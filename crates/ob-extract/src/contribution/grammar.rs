//! The verb and governor grammar: finite-verb and
//! subject shapes, governor and cancellation phrases,
//! and the walks that decide whether a date stands
//! governed, claimed, or contradicted.

use super::composition::*;
use super::dates::*;
use super::segments::*;
use super::*;

/// The indices of the nearest `count` non-empty
/// tokens before `i` in `tokens`, in ascending
/// order (round 27, R27-A, F3): a token that
/// cleans to nothing is nothing, so a positional
/// judgment that reads the words standing before
/// a position reads the nearest words that ARE
/// something — an empty token standing between a
/// preposition and its object ("on \u{200b}
/// Sunday") no more defeats the preposition's
/// detection than one standing anywhere else the
/// parser reads.
pub(super) fn prev_nonempty(tokens: &[String], mut i: usize, count: usize) -> Vec<usize> {
    let mut out = Vec::new();
    while i > 0 && out.len() < count {
        i -= 1;
        if !tokens[i].is_empty() {
            out.push(i);
        }
    }
    out.reverse();
    out
}

/// The index of the nearest non-empty token at
/// or after `i` in `tokens` (round 27, R27-A, F3):
/// the forward twin of `prev_nonempty`.
pub(super) fn next_nonempty(tokens: &[String], mut i: usize) -> Option<usize> {
    while i < tokens.len() {
        if !tokens[i].is_empty() {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The `next_nonempty` answer for every starting
/// index of `tokens`, computed in one backward
/// pass (round 31, R31-C, F4): entry `i` is the
/// nearest non-empty token at or after `i`, and
/// the entry at `tokens.len()` is None. A scan
/// that asks the question at every token — the
/// plain and group signal scans do — otherwise
/// rescans a run of tokens that clean to nothing
/// once per token in it: a de-grouped header core
/// is mostly the composition's own ";" markers,
/// which clean to nothing, so a line of thousands
/// of groups cost quadratic time in exactly this
/// lookup. The table answers the same question;
/// no answer changes.
pub(super) fn next_nonempty_table(tokens: &[String]) -> Vec<Option<usize>> {
    let mut table: Vec<Option<usize>> = vec![None; tokens.len() + 1];
    let mut next: Option<usize> = None;
    for i in (0..tokens.len()).rev() {
        if !tokens[i].is_empty() {
            next = Some(i);
        }
        table[i] = next;
    }
    table
}

/// The last word that is something in a token run
/// (round 28, R28-A, F2 = review MAJOR-1): the
/// `last()` a positional reader means — a token
/// that cleans to nothing standing at a window's
/// end ("use \u{200b} November 8", "put \u{200b}"
/// before its date) is nothing there, so the
/// window's last word is the nearest word that is
/// something before it.
pub(super) fn last_word(tokens: &[String]) -> Option<&str> {
    tokens
        .iter()
        .rev()
        .find(|w| !w.is_empty())
        .map(String::as_str)
}

/// The phrase of `len` WORDS standing at word
/// position `i` in `tokens`, and the index just
/// past its last word (round 28, R28-A, F2 =
/// review MAJOR-1 site 4): phrase matching slices
/// over the run's words that are something, not
/// over raw positions — a three-word governor
/// phrase with a nothing-token inside it ("used
/// \u{200b} to be") is a three-word phrase, and a
/// one-word phrase wearing a nothing in the old
/// positional slice ("was \u{200b} singing") is a
/// one-word phrase, judged with its one-word
/// narrowings. `tokens[i]` must itself be a word
/// that is something (every caller skips empty
/// positions before asking); fewer than `len`
/// words remaining is no phrase.
pub(super) fn word_phrase(tokens: &[String], i: usize, len: usize) -> Option<(Vec<String>, usize)> {
    if tokens.get(i).map_or(true, |w| w.is_empty()) {
        return None;
    }
    let mut words: Vec<String> = Vec::with_capacity(len);
    let mut j = i;
    while words.len() < len {
        let w = tokens.get(j)?;
        if !w.is_empty() {
            words.push(w.clone());
        }
        j += 1;
    }
    Some((words, j))
}

/// The preposition heading the phrase whose
/// object stands at `i` in `tokens`, judged over
/// the nearest non-empty words (round 27, R27-A,
/// F3 = review R27-5): the consultation's own
/// positional detection — [preposition],
/// [determiner / relative word, preposition],
/// [determiner, relative word, preposition] —
/// read through any tokens that clean to nothing
/// standing among the pattern's words.
pub(super) fn continuation_prep_before(tokens: &[String], i: usize) -> Option<usize> {
    let prev = prev_nonempty(tokens, i, 3);
    let n = prev.len();
    if n == 0 {
        return None;
    }
    let a = prev[n - 1];
    if is_continuation_preposition(tokens[a].as_str()) {
        return Some(a);
    }
    if n >= 2 {
        let b = prev[n - 2];
        if matches!(tokens[a].as_str(), "the" | "a" | "an" | "next" | "this")
            && is_continuation_preposition(tokens[b].as_str())
        {
            return Some(b);
        }
    }
    if n >= 3 {
        let c = prev[n - 3];
        if matches!(tokens[a].as_str(), "next" | "this")
            && matches!(tokens[prev[n - 2]].as_str(), "the" | "a" | "an")
            && is_continuation_preposition(tokens[c].as_str())
        {
            return Some(c);
        }
    }
    None
}

/// Whether the weekday word at `i` in `tokens`
/// stands as a subject PP's object, judged by the
/// words before it (round 27, R27-A, F3): the
/// four standing patterns — [preposition],
/// [determiner, preposition], [relative word,
/// preposition], [determiner, relative word,
/// preposition] — read over the nearest non-empty
/// words, so a token that cleans to nothing among
/// them ("the choir on \u{200b} Sunday sang")
/// defeats none of them. The subject walks'
/// weekday crossings share this one judgment.
pub(super) fn weekday_is_subject_pp_object(tokens: &[String], i: usize) -> bool {
    let prev = prev_nonempty(tokens, i, 3);
    let n = prev.len();
    if n == 0 {
        return false;
    }
    let a = tokens[prev[n - 1]].as_str();
    if is_subject_preposition(a) {
        return true;
    }
    if n >= 2 {
        let b = tokens[prev[n - 2]].as_str();
        if (matches!(a, "the" | "a" | "an" | "next" | "this") && is_subject_preposition(b)) {
            return true;
        }
    }
    if n >= 3 {
        let b = tokens[prev[n - 2]].as_str();
        let c = tokens[prev[n - 3]].as_str();
        if matches!(a, "next" | "this")
            && matches!(b, "the" | "a" | "an")
            && is_subject_preposition(c)
        {
            return true;
        }
    }
    false
}

/// The subject-PP twin of `continuation_prep_
/// before` (round 27, R27-A, F3): [preposition] or
/// [determiner, preposition] over the nearest
/// non-empty words, against the subject set.
pub(super) fn subject_prep_before(tokens: &[String], i: usize) -> Option<usize> {
    let prev = prev_nonempty(tokens, i, 2);
    let n = prev.len();
    if n == 0 {
        return None;
    }
    let a = prev[n - 1];
    if is_subject_preposition(tokens[a].as_str()) {
        return Some(a);
    }
    if n >= 2 {
        let b = prev[n - 2];
        if matches!(tokens[a].as_str(), "the" | "a" | "an")
            && is_subject_preposition(tokens[b].as_str())
        {
            return Some(b);
        }
    }
    None
}

/// Whether the weekday word at `i` in `tokens` is
/// consumed wholly as the object of a prepositional
/// phrase inside a target claim's OBJECT
/// continuation (round 24, R24-5): a subject
/// preposition stands directly before the weekday
/// (a determiner may stand between them), walking
/// BACK from that preposition across the object
/// list — noun runs, verb-form adjectives heading
/// them, determiners, conjunctions, an intervening
/// date's own words, and the earlier PPs of the
/// stack — reaches the claim's target phrase
/// without crossing an auxiliary or a finite verb,
/// and walking FORWARD from the weekday across the
/// rest of the PP stack reaches the claim's "to" /
/// "until" and the framed date it names: "(moved
/// November 1, and baked beans from Kent on Sunday
/// to November 8)", and its swapped twin "… baked
/// beans on Sunday from Kent to …". Such a weekday
/// is a modifier of the object the claim moves; it
/// spawns no relative signal, asserts no phantom
/// October 18, and is named nowhere — exactly as a
/// subject PP's weekday is consumed (R22-5/R23-5).
/// The standing "from" governance of the line is
/// untouched: the target date in these witnesses
/// reads nothing because "from" governs it, as the
/// held twin without the weekday PP does. Two
/// boundaries: a weekday PP with no target phrase
/// behind it is no claim's object continuation and
/// keeps its standing readings; and a weekday PP
/// standing in a SECOND clause — its own subject
/// and finite verb between the claim's phrase and
/// the preposition ("beans were served on Sunday")
/// — is that clause's own phrase, not a PP inside
/// the continuation, and keeps its signal.
///
/// Round 25 (R25-B, F5 = red team RT25-4, review
/// R25-2/R25-3) widened the consultation on three
/// arms, all judged by the same two walks: (a) the
/// backward walk crosses the claim's OWN clause
/// verb and auxiliaries when object material
/// stands behind them, so a claim written as a
/// full sentence ("the choir sang baked beans …
/// on Sunday to November 8") consumes exactly as
/// its subjectless twin does — the second-clause
/// boundary stands, discriminated by whether
/// object material stands between the verb and
/// the PP; (b) a RELATIVE phrase object ("on next
/// Sunday") is consulted by this same check, and
/// the forward arm accepts a relative phrase as
/// the claim's target after "to"/"until"; (c) a
/// weekday PP trailing the claim's target date is
/// judged by `weekday_pp_after_claimed_target`.
///
/// Round 28 (R28-B, F5 tail = red team RT28-W4):
/// the consultation has a second, NAMING-ONLY
/// reading, entered through
/// `weekday_consumed_in_destination_stack`: with
/// `cross_destination_to` set, the backward walk
/// also crosses a destination "to" PP standing
/// before the weekday in the stack ("… from Kent
/// to the hall on Sunday to November 8"). That
/// Sunday is the stack's own material, but it is
/// NOT consumed on the signal surfaces: its
/// occurrence bounds the target's governor window,
/// which is exactly why the target asserts in that
/// shape. The naming reading exists so the group's
/// naming can leave the Sunday's resolution
/// unnamed beside the assertion it bounds — the
/// phantom is silenced in the note, the target
/// keeps asserting. Every signal-scan caller uses
/// the standing reading (`cross_destination_to`
/// unset) and is unchanged.
pub(super) fn weekday_in_object_continuation_pp(
    raw_tokens: &[&str],
    tokens: &[String],
    i: usize,
    today: SimpleDate,
) -> bool {
    weekday_in_object_continuation_pp_crossing(raw_tokens, tokens, i, today, false)
}

/// Whether the weekday at `i` is consumed as a PP
/// inside the destination stack for NAMING
/// purposes (round 28, R28-B, F5 tail = red team
/// RT28-W4): the consultation above with its
/// backward destination crossing enabled. Read
/// only by `analyze_group`'s naming exclusion;
/// never by a signal scan.
pub(super) fn weekday_consumed_in_destination_stack(
    raw_tokens: &[&str],
    tokens: &[String],
    i: usize,
    today: SimpleDate,
) -> bool {
    weekday_in_object_continuation_pp_crossing(raw_tokens, tokens, i, today, true)
}

/// The segment start the backward walk's
/// finite-verb branch computes for the verb form
/// at `verb` (round 29, R29-B, F4): the words after
/// the nearest intervening date, exactly as that
/// branch computes it inline — factored out so the
/// verb-slot judgments that stand beside the
/// branch (the particle reading and the
/// weekday-abbreviation reading in the crossing
/// below) judge a verb form in the same segment
/// the branch itself would.
pub(super) fn verb_segment_start(raw_tokens: &[&str], verb: usize, today: SimpleDate) -> usize {
    let mut seg_start = 0usize;
    let mut s = 0usize;
    while s < verb {
        match parse_date_prefix(&raw_tokens[s..], today) {
            Some((_, used)) if s + used.max(1) < verb + 1 => {
                seg_start = s + used.max(1);
                s += used.max(1);
            }
            _ => s += 1,
        }
    }
    seg_start
}

/// Whether the word at `verb` is a finite verb
/// form the one verb test reads as finite in its
/// own segment (round 29, R29-B, F4): the backward
/// walk's finite-verb branch judgment, callable
/// from the judgments that stand beside it.
pub(super) fn finite_verb_at(
    raw_tokens: &[&str],
    tokens: &[String],
    verb: usize,
    today: SimpleDate,
) -> bool {
    if !is_finite_verb_form(&tokens[verb]) {
        return false;
    }
    let seg_start = verb_segment_start(raw_tokens, verb, today);
    verb_form_is_finite(&tokens[seg_start..], verb - seg_start)
}

/// Whether the word at `verb` is a finite verb
/// form standing behind a subject in its own
/// segment (round 29, R29-B, F4): the subject arm
/// of the one verb test, read on its own. A verb
/// whose particle is a noun-like word ("sat
/// down") is read by the full test as an
/// adjective heading that word as its noun
/// phrase — the determiner shield — so the
/// verb-slot judgments that stand beside the
/// branch read this arm for the particle shapes:
/// a subject standing before the form in its
/// segment is what makes it the clause's verb
/// there, and a word with no subject before it
/// (a PP's weekday object, a modifier) fails it.
pub(super) fn subject_verb_at(
    raw_tokens: &[&str],
    tokens: &[String],
    verb: usize,
    today: SimpleDate,
) -> bool {
    if !is_finite_verb_form(&tokens[verb]) {
        return false;
    }
    let seg_start = verb_segment_start(raw_tokens, verb, today);
    segment_subject_before(&tokens[seg_start..], verb - seg_start)
}

/// The verb-slot judgment the particle and
/// weekday-abbreviation readings share (round 29,
/// R29-B, F4): a finite verb form the one verb
/// test reads as finite, or one standing behind
/// a subject in its segment.
pub(super) fn verb_slot_finite_at(
    raw_tokens: &[&str],
    tokens: &[String],
    verb: usize,
    today: SimpleDate,
) -> bool {
    finite_verb_at(raw_tokens, tokens, verb, today)
        || subject_verb_at(raw_tokens, tokens, verb, today)
}

pub(super) fn weekday_in_object_continuation_pp_crossing(
    raw_tokens: &[&str],
    tokens: &[String],
    i: usize,
    today: SimpleDate,
    cross_destination_to: bool,
) -> bool {
    // Round 25 (R25-B, F5b = review R25-2): the
    // weekday word at `i` may be the object word of
    // a RELATIVE phrase standing as the PP's object
    // ("on next Sunday", "on the next Sunday"): the
    // same consultation judges it. Round 27
    // (R27-A, F3 = review R27-5): the detection is
    // `continuation_prep_before` — the pattern read
    // over the nearest non-empty words, so a token
    // that cleans to nothing standing between the
    // preposition and the weekday ("on \u{200b}
    // Sunday") defeats nothing.
    let Some(prep) = continuation_prep_before(tokens, i) else {
        return false;
    };
    // Backward: reach the claim's target phrase
    // across the object continuation's own material.
    let mut j = prep;
    let mut object_crossed = false;
    // Round 25 (R25-B, F5a = red team RT25-4): the
    // claim's OWN clause verb, once crossed — see
    // the finite-verb branch below.
    let mut verb_crossed = false;
    // Round 26 (R26-C, F2 mechanism 5): whether a
    // subject run (a determiner-headed noun) has
    // been crossed since the last verb crossing —
    // the discriminator between a conjoined verb
    // of the claim's own clause ("sang hymns,
    // played songs, and rehearsed pieces …": no
    // new subject stands between its verbs) and a
    // genuinely second clause's verb ("… and the
    // band played …": its own subject stands
    // between the two verbs).
    let mut subject_since_verb = false;
    // Round 27 (R27-A, F1 mechanism 5): a bare or
    // proper noun run crossed behind a crossed verb
    // is only a CANDIDATE subject — the same run is
    // the next verb's object in a conjoined verb
    // list ("sang hymns, played songs, and
    // rehearsed pieces": "hymns" stands between
    // "played" and "sang" in this walk and heads no
    // clause). The candidate arms the boundary when
    // the conjunction that joins the clauses is
    // crossed behind it: a subject stands between
    // its verb and the conjunction ("... and they
    // played songs ..."), an object stands between
    // the conjunction and its verb. Determiner /
    // possessive heads and subject pronouns arm
    // immediately, as the determiner branch always
    // has — they cannot head an object run in this
    // walk's position.
    let mut pending_subject = false;
    // Round 27 (R27-A, F1 mechanism 6): whether the
    // walk ended on the second-clause boundary
    // specifically — the one ending the forward
    // arm's every-conjunct judgment below revisits.
    let mut boundary_break = false;
    // Round 28 (R28-B, F4 + F5): the forward arm is
    // judged once, here, and read by three sites
    // below: the finite-verb branch (the verb-slot
    // rule), the boundary fallback (the conjoined
    // from-crossing), and the consultation's own
    // final judgment — one judgment, not three.
    let (forward_ok, forward_conjuncts, forward_from) =
        forward_walk_continuation(raw_tokens, tokens, i, today);
    // Round 29 (R29-B, F4 = red team RT29-W1):
    // THE VERB-SLOT RULE, completed — the slot's
    // judgment covers its whole verbal material.
    // Two states the walk below reads:
    // - `particle_crossed`: a verb's own particle
    //   stands in the particle position — a
    //   preposition-form word directly after a
    //   finite verb, directly before the weekday
    //   PP ("came in on Sunday", "went on on
    //   Sunday"). The particle is part of the
    //   verb, not the head of the next PP: the two
    //   nearest words before the PP's preposition
    //   are [verb, particle] and the one verb test
    //   reads the first as the clause's finite
    //   verb. Judged once, here, on the same
    //   segment the finite-verb branch would use.
    // - `conjunction_only_since_verb`: since the
    //   last verb the walk crossed, nothing but
    //   the conjunction and its slot words (a
    //   "then", nothings) has been crossed — the
    //   shape of a conjunction of bare verbs
    //   ("sang and danced"), where a further bare
    //   verb is another conjunct of the same verb
    //   slot, not a second clause's verb. Any
    //   other material crossed since the verb —
    //   an object, a determiner, a PP, an
    //   auxiliary — clears it, and a subject run
    //   crossed since the verb keeps the standing
    //   second-clause boundary through
    //   `subject_since_verb` as it always has.
    let mut particle_crossed = false;
    let mut verb_particle_crossed = false;
    let mut conjunction_only_since_verb = false;
    {
        let prev = prev_nonempty(tokens, prep, 2);
        if prev.len() == 2 {
            let (verb_at, particle_at) = (prev[0], prev[1]);
            let pw = tokens[particle_at].as_str();
            if pw != "until"
                && is_continuation_preposition(pw)
                && verb_slot_finite_at(raw_tokens, tokens, verb_at, today)
            {
                particle_crossed = true;
            }
        }
    }
    let reached_target = loop {
        if j == 0 {
            break false;
        }
        if is_target_phrase(&tokens[j - 1..j]) || (j >= 2 && is_target_phrase(&tokens[j - 2..j])) {
            break true;
        }
        let w = tokens[j - 1].as_str();
        // Round 29 (R29-B, F4 = red team RT29-W1):
        // a weekday-abbreviation word that is also
        // a finite verb form — "sat", the one such
        // word — standing in the verb slot is the
        // clause's verb, not an earlier PP's
        // weekday object: its particle or object
        // stands crossed behind it ("sat down"),
        // its verb's particle was crossed on the
        // way in ("sat on on Sunday"), or it
        // stands as a bare conjunct behind a
        // crossed verb ("sat and sang"), or with
        // a conjunct standing before it ("sang
        // and sat"). The subject arm of the verb
        // test must read it as a clause's verb — a
        // subject standing before it in its
        // segment; the full test's determiner
        // shield reads "sat down" as an adjective
        // heading its particle, which is the
        // misreading being corrected. A weekday
        // object standing in a PP stack ("… on
        // sat from Kent on Sunday") has no subject
        // before it in that position and keeps
        // the weekday reading below.
        // The mirror conjunct position: the
        // conjunction and a finite conjunct stand
        // before this verb ("sang and sat on
        // Sunday") — the conjunct's own index, when
        // it stands there, whose subject is the
        // conjunction's shared subject.
        let conjunct_before_at: Option<usize> = {
            let prev = prev_nonempty(tokens, j - 1, 2);
            if prev.len() == 2
                && matches!(tokens[prev[1]].as_str(), "and" | "or")
                && verb_slot_finite_at(raw_tokens, tokens, prev[0], today)
            {
                Some(prev[0])
            } else {
                None
            }
        };
        let weekday_verb = weekday_word(w).is_some()
            && is_finite_verb_form(w)
            && (object_crossed
                || verb_particle_crossed
                || (verb_crossed
                    && matches!(tokens.get(j).map(String::as_str), Some("and") | Some("or")))
                || conjunct_before_at.is_some())
            && (subject_verb_at(raw_tokens, tokens, j - 1, today)
                || conjunct_before_at
                    .is_some_and(|v| subject_verb_at(raw_tokens, tokens, v, today)));
        if w.is_empty() {
            // Round 26 (R26-C, M1 = review R26-6):
            // a token that cleans to nothing (a
            // standalone zero-width / format token,
            // its "*" twin) is nothing at every site
            // the claim scan reads — the walk
            // crosses it without judging it.
            j -= 1;
        } else if matches!(w, "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w) {
            // Round 26 (R26-C, F2 mechanism 5): a
            // determiner crossed behind a verb the
            // walk already crossed marks that a
            // subject run has been crossed since —
            // a further finite verb beyond it is a
            // second clause's, not a conjoined
            // verb of the claim's own clause (see
            // the finite-verb branch).
            if verb_crossed {
                subject_since_verb = true;
            }
            conjunction_only_since_verb = false;
            j -= 1;
        } else if matches!(w, "and" | "or") {
            // Round 27 (R27-A, F1 mechanism 5): the
            // conjunction confirms a candidate
            // subject crossed behind the last verb —
            // see `pending_subject` above.
            if pending_subject {
                subject_since_verb = true;
                pending_subject = false;
            }
            j -= 1;
        } else if is_tolerated_adverb(w) || matches!(w, "never" | "ever" | "not") {
            // Round 26 (R26-C, F2 mechanism 2 =
            // review R26-2): the auxiliary stack's
            // adverbs cross as the -ly forms cross
            // (through the noun-like branch below)
            // — the FLAT class ("still", "really",
            // "never", "just", "already", "soon",
            // "also", "now") stands between an
            // auxiliary and its verb form exactly
            // as a -ly adverb does ("the choir was
            // still singing hymns …"), and the walk
            // judges the whole stack, not the word
            // immediately behind the verb form.
            j -= 1;
        } else if is_continuation_preposition(w) {
            // Round 27 (R27-A, F1 mechanism 7):
            // "until" is the claim's target
            // preposition, never object material —
            // the forward arm learned this in round
            // 26, and the backward walk crosses it
            // the same way: never. A weekday PP
            // standing after an "until" target is
            // judged by the trailing arm below, which
            // this break reaches; crossing "until"
            // and its date as noun-like material
            // reached the claim phrase by the wrong
            // road and the trailing judgment never
            // ran.
            if w == "until" {
                break false;
            }
            if !object_crossed && particle_crossed {
                // Round 29 (R29-B, F4 = red team
                // RT29-W1): the verb's own particle,
                // judged before the walk (see
                // `particle_crossed` above) — it is
                // part of the verb, not the head of
                // the next PP, so it crosses with no
                // object behind it, exactly as the
                // verb it belongs to will.
                particle_crossed = false;
                verb_particle_crossed = true;
                j -= 1;
                continue;
            }
            if !object_crossed {
                break false;
            }
            conjunction_only_since_verb = false;
            // Round 26 (R26-C, F2 mechanism 1 =
            // review R26-1): a PP inside the
            // continuation is crossed WITHOUT
            // resetting the object material already
            // crossed — before this round the reset
            // made the verb behind a PP stack read
            // as standing directly before the
            // weekday PP ("beans were served from
            // Kent on Sunday": the "from" crossing
            // erased Kent, and "served" misread as
            // a second clause's verb). The one
            // exception is "by": a by-phrase is the
            // passive's agent phrase, not object
            // material, and crossing it keeps the
            // standing reset — a by-phrase alone
            // does not flip the pinned alternatives.
            if w == "by" {
                object_crossed = false;
            }
            j -= 1;
        } else if weekday_word(w).is_some() && !weekday_verb {
            // An earlier PP's weekday object in the
            // stack, its preposition (or its
            // determiner, or its relative word)
            // standing just before it.
            let object_ok = (j >= 2 && is_continuation_preposition(tokens[j - 2].as_str()))
                || (j >= 3
                    && matches!(tokens[j - 2].as_str(), "the" | "a" | "an" | "next" | "this")
                    && is_continuation_preposition(tokens[j - 3].as_str()))
                // Round 27 (R27-A, F1 mechanism 7):
                // the relative object wearing its
                // determiner — "on the next Sunday"
                // standing as an earlier PP's object
                // in the stack — crosses by the same
                // pattern the subject walks have
                // always carried (prep at j - 4,
                // determiner at j - 3, relative word
                // at j - 2); without it a trailing
                // run of two relative PPs escaped the
                // re-judgment its bare and mixed
                // twins pass.
                || (j >= 4
                    && matches!(tokens[j - 2].as_str(), "next" | "this")
                    && matches!(tokens[j - 3].as_str(), "the" | "a" | "an")
                    && is_continuation_preposition(tokens[j - 4].as_str()));
            if !object_ok {
                break false;
            }
            object_crossed = true;
            conjunction_only_since_verb = false;
            j -= 1;
        } else if (matches!(w, "next" | "this")
            && tokens
                .get(j)
                .is_some_and(|next| weekday_word(next).is_some()))
            || (noun_like_word(w) && !is_finite_verb_form(w))
        {
            // The relative word of an earlier PP's
            // relative-phrase object, or a noun-like
            // object word: the continuation's own
            // material either way.
            // Round 27 (R27-A, F1 mechanism 5): a
            // noun-like word crossed behind a crossed
            // verb is a candidate subject (see
            // `pending_subject`) — except the
            // relative word itself, which is an
            // earlier PP's object word, never a
            // subject. A subject PRONOUN arms the
            // boundary at once: in this walk's
            // position a pronoun heads the clause it
            // stands in ("and they played"), it does
            // not object a verb of the claim's list.
            let relative_word = matches!(w, "next" | "this")
                && tokens
                    .get(j)
                    .is_some_and(|next| weekday_word(next).is_some());
            if verb_crossed && !relative_word {
                if SUBJECT_PRONOUNS.contains(&w) {
                    subject_since_verb = true;
                } else {
                    pending_subject = true;
                }
            }
            object_crossed = true;
            conjunction_only_since_verb = false;
            j -= 1;
        } else if AUXILIARY_WORDS.contains(&w) {
            // Round 25 (R25-B, F5a): an auxiliary
            // standing behind object material the
            // walk has already crossed — or behind
            // the clause verb it carries — is the
            // claim's own clause auxiliary ("the
            // choir was singing baked beans on
            // Sunday …"): the walk crosses it and
            // judges its subject like any other part
            // of the claim's clause. An auxiliary
            // with no object material behind it is a
            // second clause's, and the walk ends.
            if !object_crossed && !verb_crossed {
                break false;
            }
            verb_crossed = true;
            object_crossed = false;
            conjunction_only_since_verb = false;
            j -= 1;
        } else if is_finite_verb_form(w) {
            // A verb-form adjective heading the
            // object run ("baked beans") is the
            // continuation's own material. The verb
            // test judges the word in its own
            // segment — the words after the nearest
            // intervening date, exactly as the claim
            // reach's after-date judgment resets its
            // segment there (R18-8): an intervening
            // date's own words are not the verb
            // form's subject.
            let mut seg_start = 0usize;
            let mut s = 0usize;
            while s < j - 1 {
                match parse_date_prefix(&raw_tokens[s..], today) {
                    Some((_, used)) if s + used.max(1) < j => {
                        seg_start = s + used.max(1);
                        s += used.max(1);
                    }
                    _ => s += 1,
                }
            }
            if verb_form_is_finite(&tokens[seg_start..], j - 1 - seg_start) {
                // Round 25 (R25-B, F5a = red team
                // RT25-4): a verb-form word the one
                // verb test reads as finite ends the
                // continuation ONLY when it stands
                // directly before the preposition —
                // no object material crossed behind
                // it — which is the pinned second
                // clause's shape ("beans were served
                // on Sunday", "choir sang on
                // Sunday": the clause's verb is the
                // last word before its PP). When
                // object material HAS been crossed
                // behind the verb ("the choir sang
                // hymns from Kent on Sunday", "…
                // sang baked beans on Sunday from
                // Kent"), the verb is the claim's
                // OWN clause verb and the weekday PP
                // stands inside ITS object
                // continuation: the walk crosses the
                // verb once, then crosses its subject
                // by the same branches as any other
                // continuation material, and only the
                // target phrase beyond the subject
                // completes the reach. A second
                // finite verb ends the walk.
                // Round 26 (R26-C, F2 mechanism 1):
                // a PASSIVE verb standing directly
                // before the weekday PP is the
                // claim's own verb when a further PP
                // intervenes between the weekday PP
                // and the claim's target — "beans
                // were served on Sunday from Kent to
                // November 8": the PP stack after the
                // weekday is the continuation, and
                // the passive clause is the claim's
                // clause, exactly as its base-order
                // twin (crossed under the keep rule
                // above) reads. With no intervening
                // PP the pinned alternatives stand
                // ("beans were served on Sunday to
                // November 8"), and a by-phrase alone
                // does not flip them.
                // Round 26 (R26-C, F2 mechanism 5): a
                // second finite verb crossed behind
                // object material, with NO subject
                // run crossed since the last verb, is
                // a conjoined verb of the claim's own
                // clause ("…, played songs, and
                // rehearsed pieces from Kent on
                // Sunday …" — one subject, a verb
                // list); a subject run crossed since
                // the last verb makes it a second
                // clause's verb, and the walk ends
                // as it always has.
                let passive_stack_cross = !object_crossed
                    && !verb_crossed
                    && passive_aux_before(tokens, j - 1)
                    && forward_has_intervening_pp(raw_tokens, tokens, i, today);
                // Round 28 (R28-B, F4 = red team
                // RT28-W1): THE VERB-SLOT RULE, stated
                // once — the from-crossing does not
                // depend on the verb's shape. A finite
                // verb standing in the clause's verb
                // slot directly before the weekday PP,
                // carrying no object and no auxiliary
                // ("the choir sang on Sunday from
                // Kent …"), is the claim's own clause
                // verb exactly as the same verb
                // carrying an object ("sang hymns") or
                // an auxiliary ("was travelling") is,
                // when the weekday's forward stack
                // reaches the claim's target through a
                // "from" PP: the claim-phrase span and
                // the from-scope are the same whatever
                // stands in the verb slot. (The two
                // verbs that already held — "drove" /
                // "rode" — hold because they stand in
                // no finite-form list, so the walk
                // never saw a verb there at all; that
                // accident is the shape of this rule,
                // not an exception list.) Without a
                // from PP in the stack the standing
                // readings hold untouched: no target
                // construction, a destination-only or
                // static-PP stack, and the passive
                // shape above keep the alternatives /
                // target readings the bare verb has
                // always given there.
                let bare_from_cross =
                    !object_crossed && !verb_crossed && forward_ok && forward_from;
                // Round 29 (R29-B, F4 = red team
                // RT29-W1): the same verb-slot rule
                // over a conjunction — a conjunction
                // of bare finite verbs is the clause's
                // verb exactly as one bare verb is.
                // A further bare verb crossed behind
                // a crossed verb with nothing but the
                // conjunction and its slot words
                // between them (`conjunction_only_
                // since_verb`, and no subject run
                // crossed since) is another conjunct
                // of the same slot, under the same
                // from-scope the single bare verb
                // crosses under. An object anywhere
                // in the slot already crosses by the
                // object rule above, and a participle
                // standing in the slot is not a bare
                // finite verb of the conjunction —
                // both keep their standing readings.
                let conjoined_bare_cross = !object_crossed
                    && verb_crossed
                    && conjunction_only_since_verb
                    && !subject_since_verb
                    && forward_ok
                    && forward_from;
                if !object_crossed
                    && !passive_stack_cross
                    && !bare_from_cross
                    && !conjoined_bare_cross
                {
                    break false;
                }
                if verb_crossed && subject_since_verb {
                    boundary_break = true;
                    break false;
                }
                verb_crossed = true;
                object_crossed = false;
                subject_since_verb = false;
                pending_subject = false;
                conjunction_only_since_verb = true;
                j -= 1;
            } else {
                object_crossed = true;
                conjunction_only_since_verb = false;
                j -= 1;
            }
        } else if cross_destination_to
            && w == "to"
            && object_crossed
            && destination_to_end(raw_tokens, tokens, j - 1, today).is_some()
        {
            // Round 28 (R28-B, F5 tail = red team
            // RT28-W4): the NAMING consultation
            // only (see
            // `weekday_in_object_continuation_pp`):
            // a destination "to" PP standing BEFORE
            // the weekday in the stack is crossed
            // backward exactly as the forward arm
            // crosses it forward — "… from Kent to
            // the hall on Sunday to November 8": the
            // destination is the stack's own
            // material, its object run already
            // crossed behind this "to", and the
            // Sunday after it stands inside the same
            // continuation. The shared
            // `destination_to_end` rule judges the
            // "to": a "to" naming a date (the
            // claim's target standing before a
            // trailing PP) is never a destination,
            // and a "to" with no destination object
            // crossed behind it ends the walk as
            // every uncrossed "to" always has.
            j -= 1;
        } else {
            break false;
        }
    };
    if !reached_target {
        // Round 25 (R25-B, F5c = review R25-3): the
        // PP may stand AFTER the claim's target date
        // instead of inside the continuation before
        // it — the backward walk dies at that date's
        // "to", so the trailing arm is tried here.
        if weekday_pp_after_claimed_target(raw_tokens, tokens, prep, i, today, cross_destination_to)
        {
            return true;
        }
        // Round 27 (R27-A, F1 mechanism 6): the walk
        // ended on the conjoined-clause boundary — a
        // subject run armed behind a crossed verb —
        // which is the pinned alternatives reading when
        // the claim's target stands in the weekday's
        // own conjunct. But a boundary arms at EVERY
        // conjunct: when the forward arm reaches the
        // target only by crossing a further full
        // conjunct (conjunction + subject run + verb)
        // standing after the weekday PP, the PP stands
        // inside the continuation the same way — the
        // clause after it is the claim's continuation
        // carrying its target, and the Sunday is
        // consumed. Without that crossing the boundary
        // stands and the alternatives reading holds.
        if boundary_break {
            // Round 28 (R28-B, F5a = red team
            // RT28-W2): the boundary also falls when
            // the forward arm reaches the target
            // through a "from" PP in the weekday's
            // own conjunct — the single-clause
            // from-crossing composed across the
            // clause boundary. In a single clause the
            // same stack consumes ("the band played
            // songs on Sunday from Kent to November
            // 8" alone reads silence); the clause
            // boundary behind the walk does not
            // change the from-scope ahead of it.
            // Stacks with no "from" — no continuation,
            // a destination PP alone, a static PP —
            // keep the pinned alternatives, and a
            // "from" standing BEFORE the weekday is
            // not in this arm's stack at all, so the
            // pinned G2-C1 reading stands.
            return forward_ok && (forward_conjuncts >= 1 || forward_from);
        }
        return false;
    }
    forward_ok
}

/// The consultation's forward arm, standing alone
/// (round 27, R27-A): cross the rest of the PP
/// stack after the weekday at `i` to the claim's
/// "to" / "until" and the framed date (or relative
/// phrase) it names. Returns whether the target
/// was reached, and how many FULL conjoined
/// clauses (conjunction + subject run + verb) the
/// crossing had to cross to reach it — the count
/// the mechanism-6 boundary fallback in
/// `weekday_in_object_continuation_pp` reads.
/// Round 27 (R27-A, F1 mechanisms 2 and 4): the
/// slot immediately after the weekday PP is
/// judged by the auxiliary stack's own word class
/// (`aux_slot_word`) — a flat adverb standing
/// there ("on Sunday still from Kent …") is the
/// stack's adverb one position later, and it
/// crosses as its -ly twin does; and the stack's
/// PP crossings cover the destination and
/// accompaniment prepositions (see the arms
/// below), without disturbing the static-PP
/// readings.
/// Round 28 (R28-B, F4 + F5): the arm also reports
/// whether the stack it crossed to the target
/// carried a "from" PP — the from-scope the
/// backward walk's verb-slot rule and the
/// conjoined-boundary fallback both read (see
/// `weekday_in_object_continuation_pp`). A stack
/// of static, destination, or accompaniment PPs
/// alone does not set it.
pub(super) fn forward_walk_continuation(
    raw_tokens: &[&str],
    tokens: &[String],
    i: usize,
    today: SimpleDate,
) -> (bool, usize, bool) {
    let mut conjuncts_crossed = 0usize;
    let mut from_crossed = false;
    let mut k = i + 1;
    // The post-PP slot: the auxiliary stack's slot
    // words, judged by the stack's own class.
    while tokens.get(k).is_some_and(|w| aux_slot_word(w)) {
        k += 1;
    }
    let forward_ok = loop {
        // Round 26 (R26-C, F2 mechanism 5 = red
        // team RT26-7): "until" is the claim's
        // target preposition, never object
        // material — before this round the
        // noun-like crossing below read "until" as
        // a noun-like word (it stands in no
        // exclusion list `noun_like_word` checks),
        // crossed it and its date, and died at the
        // end of the token run, so an "until" target
        // consumed only when the weekday PP stood
        // directly before it (where a different
        // site treats the weekday as the target
        // date's framing weekday). "until" now
        // terminates the crossing exactly as "to"
        // does, at every stack depth and order.
        // Round 26 (R26-C, M1): a token that cleans
        // to nothing is crossed without judging it,
        // on this arm too.
        // The stack's slot words cross wherever in
        // the stack they stand (the post-PP slot
        // above is their first position, not their
        // only one).
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
                let mut m = k + 1;
                while tokens
                    .get(m)
                    .is_some_and(|w| w.is_empty() || matches!(w.as_str(), "the" | "a" | "an"))
                {
                    m += 1;
                }
                // Round 25 (R25-B, F5b = review
                // R25-2): the claim's target after
                // "to"/"until" may itself be a
                // relative phrase ("… on Sunday to
                // next Sunday"): the claim's target
                // either way, so the PP object before
                // it is consumed all the same.
                if matches!(
                    tokens.get(m).map(String::as_str),
                    Some("next") | Some("this")
                ) && tokens.get(m + 1).is_some_and(|w| weekday_word(w).is_some())
                {
                    break true;
                }
                if raw_tokens
                    .get(m..)
                    .is_some_and(|rest| parse_date_prefix(rest, today).is_some())
                {
                    break true;
                }
                // Round 27 (R27-A, F1 mechanism 4): a
                // "to" that names no date may head a
                // DESTINATION PP instead of the
                // claim's target — "… from Kent to
                // the hall to November 8". Round 28
                // (R28-B, F5 = red team RT28-W3): the
                // discriminator is the shared
                // place-shaped NP rule
                // (`destination_to_end`) — determiner-
                // headed, possessive-headed, or a bare
                // place noun/name — with a further
                // target construction after it. Cross
                // the PP and judge what follows by the
                // same arms; a "to" heading no
                // place-shaped NP, or with nothing
                // after its NP, ends the arm exactly
                // as before.
                if tokens[k] == "to" {
                    if let Some(n) = destination_to_end(raw_tokens, tokens, k, today) {
                        k = n;
                        continue;
                    }
                }
                break false;
            }
            Some(w) if is_continuation_preposition(w) => {
                // Round 28 (R28-B, F4 + F5): a "from"
                // PP crossed in the stack is the
                // from-scope the consultation's other
                // arms read.
                if w == "from" {
                    from_crossed = true;
                }
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
                if let Some((_, used)) = raw_tokens
                    .get(m..)
                    .and_then(|rest| parse_date_prefix(rest, today))
                {
                    k = m + used.max(1);
                    continue;
                }
                // Round 27 (R27-A, F1 mechanism 4):
                // the object run stops at the next
                // PP's preposition — "into" / "with"
                // / "through" and their kin are
                // noun-like by the word test, so the
                // run used to swallow a following
                // destination or accompaniment PP's
                // preposition and head noun and die
                // at its determiner. Each PP of the
                // stack is now crossed by its own
                // iteration of this arm.
                let object_start = m;
                while tokens.get(m).is_some_and(|w| {
                    w.is_empty()
                        || (noun_like_word(w)
                            && !is_finite_verb_form(w)
                            && w.as_str() != "until"
                            && !is_continuation_preposition(w))
                }) {
                    m += 1;
                }
                if m == object_start {
                    break false;
                }
                k = m;
            }
            Some("and") | Some("or") => {
                // Round 26 (R26-C, F2 mechanism 5 =
                // red team RT26-7): a conjoined
                // clause inside the continuation
                // does not end the forward arm —
                // "… on Sunday and the band played
                // from Kent to November 8" and "…
                // from Kent and played songs to
                // November 8" carry the claim's
                // target in the second conjunct, and
                // the weekday PP before the
                // conjunction stands inside the same
                // continuation. Cross the conjunction,
                // the conjunct's subject run (a
                // determiner-headed noun run, when
                // one stands there), its auxiliary
                // stack, and its verb form, then
                // judge what follows by the same
                // arms as before.
                let mut m = k + 1;
                // Round 28 (R28-A, F2): the conjunct's
                // runs cross nothings, as every other
                // run in this walk does.
                while tokens.get(m).is_some_and(|w| {
                    w.is_empty()
                        || matches!(w.as_str(), "the" | "a" | "an")
                        || POSSESSIVE_DETERMINERS.contains(&w.as_str())
                }) {
                    m += 1;
                }
                while tokens.get(m).is_some_and(|w| {
                    w.is_empty()
                        || (noun_like_word(w) && !is_finite_verb_form(w) && w.as_str() != "until")
                }) {
                    m += 1;
                }
                let mut v = m;
                while tokens.get(v).is_some_and(|w| {
                    w.is_empty() || AUXILIARY_WORDS.contains(&w.as_str()) || aux_slot_word(w)
                }) {
                    v += 1;
                }
                if tokens.get(v).is_some_and(|w| is_finite_verb_form(w)) {
                    // Round 27 (R27-A, F1 mechanism
                    // 6): a FULL conjunct crossed —
                    // subject run and verb — counted
                    // for the boundary fallback in the
                    // consultation.
                    conjuncts_crossed += 1;
                    k = v + 1;
                    continue;
                }
                if m > k + 1 {
                    k = m;
                    continue;
                }
                break false;
            }
            _ => break false,
        }
    };
    (forward_ok, conjuncts_crossed, from_crossed)
}

/// Round 25 (R25-B, F5c = review R25-3): a weekday
/// PP standing AFTER a target claim's target date
/// ("moved November 1, baked beans from Kent to
/// November 8 on Sunday") is the claim's own
/// trailing phrase: consumed, so the line reads
/// exactly as the twin without the trailing PP
/// reads. The judgment is the twin judgment — the
/// PP is consumed exactly when the same PP
/// standing at the end of the object continuation,
/// just before the claim's "to", would be consumed
/// by the check above — so the trailing shape is
/// re-judged on a token run rebuilt in that order.
/// A line carrying no claim construction (no
/// "to"/"until" and its framed date behind the PP,
/// with the claim's target phrase behind that)
/// never reaches the re-judgment: the claim-free
/// "(baked beans to November 8 on Sunday)" keeps
/// both dates as alternatives, as pinned at R24-B.
///
/// Round 29 (R29-B, F7 = red team RT29-N1): the
/// re-judgment carries the caller's own reading —
/// when the NAMING consultation (`cross_
/// destination_to` set) asks whether a trailing
/// Sunday was consumed, the rebuilt run is judged
/// with the destination crossing enabled, so a
/// trailing Sunday standing after a target whose
/// construction carries a destination stack
/// ("… to the hall to November 8 on Sunday") is
/// consumed for naming exactly as the same Sunday
/// standing inside the stack is. The signal
/// reading is unchanged: its callers pass the
/// flag unset and the rebuilt run is judged by
/// the standing consultation, as it always has
/// been.
pub(super) fn weekday_pp_after_claimed_target(
    raw_tokens: &[&str],
    tokens: &[String],
    prep: usize,
    i: usize,
    today: SimpleDate,
    cross_destination_to: bool,
) -> bool {
    // Round 26 (R26-C, F2 mechanism 3 = review
    // R26-3): the PP may be one of a trailing RUN
    // — "… to November 8 on Sunday on Sunday", its
    // comma twin, and the mixed bare+relative
    // pairs — and the R25-B arm judged only the
    // single PP whose preposition the claimed
    // date ends directly before, so a run's later
    // PP escaped and asserted its resolution with
    // the claimed date demoted. The run is judged
    // as the single trailing PP is: walk back from
    // this PP's preposition over every complete
    // PP of the run (a continuation preposition
    // plus its object — a weekday, a relative
    // phrase, a noun run, or a framed date) to
    // where the claimed date ends, and re-judge
    // the whole run in its forward-twin position.
    // A different second weekday ("… on Sunday on
    // Monday") is no part of any Sunday's run and
    // keeps its standing reading: the run walk
    // only ever starts at a Sunday PP's own
    // preposition, and the earlier PP of the run
    // is re-judged by its own consultation.
    let mut run_start = prep;
    loop {
        let cur = run_start;
        // One PP ending exactly at `cur`, if one
        // stands there: its preposition at `q`,
        // its object in `tokens[q + 1..cur]`.
        let mut found: Option<usize> = None;
        // Weekday / relative-phrase objects, by
        // shape length (longest first).
        // Round 28 (R28-A, F2): the shapes are
        // judged on the object's WORDS — a token
        // that cleans to nothing standing inside
        // the object ("until \u{200b} November 8 on
        // Sunday", "… on \u{200b} Sunday") is
        // nothing in it, so the candidate
        // preposition is the word standing just
        // before the object's words, however many
        // nothings stand between them.
        let word_pos: Vec<usize> = (0..cur).filter(|&p| !tokens[p].is_empty()).collect();
        for objlen in [3usize, 2, 1] {
            if word_pos.len() < objlen + 1 {
                continue;
            }
            let q = word_pos[word_pos.len() - objlen - 1];
            if !is_continuation_preposition(tokens[q].as_str()) {
                continue;
            }
            let obj: Vec<&str> = word_pos[word_pos.len() - objlen..]
                .iter()
                .map(|&p| tokens[p].as_str())
                .collect();
            let shape_ok = match obj.len() {
                1 => weekday_word(obj[0]).is_some(),
                2 => {
                    matches!(obj[0], "the" | "a" | "an" | "next" | "this")
                        && weekday_word(obj[1]).is_some()
                }
                3 => {
                    matches!(obj[0], "the" | "a" | "an")
                        && matches!(obj[1], "next" | "this")
                        && weekday_word(obj[2]).is_some()
                }
                _ => false,
            };
            if shape_ok {
                found = Some(q);
                break;
            }
        }
        if found.is_none() {
            // A framed-date object. Round 27
            // (R27-A, F1 mechanism 7): "until" is
            // never a run PP's preposition — an
            // "until" + framed date ending at `cur`
            // is the claim's own target standing
            // before the run, not a PP of the run;
            // crossing it as one made the arm look
            // for a second date before the "from"
            // phrase and die, so every "until"
            // trailing shape escaped the judgment
            // its "to" twin passes.
            for q in (0..cur).rev() {
                if !is_continuation_preposition(tokens[q].as_str()) || tokens[q].as_str() == "until"
                {
                    continue;
                }
                if let Some((_, used)) = parse_date_prefix(&raw_tokens[q + 1..], today) {
                    // Round 28 (R28-A, F2): the date
                    // ends at `cur` when only nothings
                    // stand between its end and `cur`.
                    let end = q + 1 + used.max(1);
                    if end <= cur && tokens[end..cur].iter().all(|w| w.is_empty()) {
                        found = Some(q);
                        break;
                    }
                }
            }
        }
        if found.is_none() {
            // A noun-run object: the nearest
            // preposition whose object slice is
            // determiners and noun-like words. An
            // "until" inside the slice ends it, as
            // above (R27-A mechanism 7): the slice
            // "Kent until November 8" is a target
            // construction, not a noun run.
            for q in (0..cur).rev() {
                if q + 1 >= cur {
                    continue;
                }
                if !is_continuation_preposition(tokens[q].as_str()) || tokens[q].as_str() == "until"
                {
                    continue;
                }
                let obj = &tokens[q + 1..cur];
                if obj.iter().all(|w| {
                    // Round 28 (R28-A, F2): a nothing
                    // inside the noun run is nothing
                    // in it.
                    w.is_empty()
                        || matches!(w.as_str(), "the" | "a" | "an")
                        || POSSESSIVE_DETERMINERS.contains(&w.as_str())
                        || (noun_like_word(w) && !is_finite_verb_form(w) && w.as_str() != "until")
                }) {
                    found = Some(q);
                    break;
                }
            }
        }
        match found {
            Some(q) => run_start = q,
            None => break,
        }
    }
    // The framed date span ending exactly at the
    // run's first preposition.
    let mut date_start = None;
    for cand in (0..run_start).rev() {
        if let Some((_, used)) = parse_date_prefix(&raw_tokens[cand..], today) {
            // Round 28 (R28-A, F2): the date ends at
            // the run when only nothings stand
            // between its end and the run's first
            // preposition.
            let end = cand + used.max(1);
            if end <= run_start && tokens[end..run_start].iter().all(|w| w.is_empty()) {
                date_start = Some(cand);
                break;
            }
        }
    }
    let Some(date_start) = date_start else {
        return false;
    };
    // The claim's "to"/"until" standing directly
    // before that date, behind optional determiners.
    // Round 28 (R28-A, F2): the walk back crosses
    // nothings with the determiners — "until
    // \u{200b} November 8" claims its date exactly
    // as the clean form does.
    let mut t = date_start;
    while t > 0
        && (tokens[t - 1].is_empty() || matches!(tokens[t - 1].as_str(), "the" | "a" | "an"))
    {
        t -= 1;
    }
    if t == 0 || !matches!(tokens[t - 1].as_str(), "to" | "until") {
        return false;
    }
    let to_pos = t - 1;
    // Rebuild the run with the PP (or the whole
    // trailing run) standing where its forward
    // twin stands: at the end of the continuation,
    // just before the "to", with the claimed date
    // after it.
    let mut syn_tokens: Vec<String> = tokens[..to_pos].to_vec();
    syn_tokens.extend_from_slice(&tokens[run_start..=i]);
    syn_tokens.push("to".to_string());
    syn_tokens.extend_from_slice(&tokens[date_start..run_start]);
    let mut syn_raw: Vec<&str> = raw_tokens[..to_pos].to_vec();
    syn_raw.extend_from_slice(&raw_tokens[run_start..=i]);
    syn_raw.push("to");
    syn_raw.extend_from_slice(&raw_tokens[date_start..run_start]);
    let syn_i = to_pos + (i - run_start);
    weekday_in_object_continuation_pp_crossing(
        &syn_raw,
        &syn_tokens,
        syn_i,
        today,
        cross_destination_to,
    )
}

/// Whether `word` can head the noun phrase a
/// verb form would modify standing directly
/// before it (round 21, R21-6): a noun-like word
/// that is not itself adverbial — an -ly word
/// after a verb form is that verb's adverb
/// ("played beautifully"), never its noun.
pub(super) fn noun_phrase_word(word: &str) -> bool {
    noun_like_word(word) && !adverbial_word(word)
}

/// The ONE verb test for every verb-form word in
/// a segment (round 20, R20-4; round 21, R21-6,
/// the verb test v2): a verb-form word — a
/// generic -ed form, a listed stem form, an
/// irregular form, an -ing form — is a finite
/// verb when a subject stands before it in its
/// segment (the run rule of
/// `segment_subject_before`: "the big choir sang",
/// "choir members sang", "Pastor Lee sang"), or
/// when no subject stands before it but the word
/// cannot head a noun phrase there: an irregular
/// past form ("sang hymns" — an irregular form
/// has no adjective use the test can read, the
/// distinction between "sang" / "sung" and
/// "baked" that the spelling itself carries), or
/// a form followed by a determiner or an adverb
/// ("folded the bulletins", "played
/// beautifully"), or a bare form with no noun
/// phrase around it at all ("then played"). It
/// reads as a modifier only when a bare noun-like
/// word follows it directly with no subject
/// before it ("baked beans", "planned dinner",
/// "planning meeting"), or when it stands
/// postposed at its segment's end after the noun
/// it modifies ("dinner rolls, baked"). A verb
/// form standing directly after a determiner or
/// a possessive is that noun phrase's own
/// material, as it has always been ("the baked
/// beans").
pub(super) fn verb_form_is_finite(segment: &[String], off: usize) -> bool {
    if off > 0 {
        let prev = segment[off - 1].as_str();
        // Round 23 (R23-8): the immediate shield is
        // scoped by the past-only split exactly as
        // the spanned shield below is — a past-only
        // form ("sang", "rang", "took") has no
        // adjective use in ANY position, the
        // determiner directly before it
        // notwithstanding, so "the sang hymns" reads
        // as a clause exactly as "the big sang
        // hymns" already did. Participle and regular
        // forms keep the shield ("the taken seats",
        // "the baked beans").
        if (matches!(prev, "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&prev))
            && !IRREGULAR_PAST_ONLY_FORMS.contains(&segment[off].as_str())
        {
            return false;
        }
    }
    // The noun phrase it would head may itself
    // wear a weekday modifier (round 20, R20-6's
    // reading): "printed Sunday bulletins" heads
    // the same noun phrase "printed bulletins"
    // does.
    // Round 27 (R27-A, F3): the phrase's words are
    // the next words that are something after the
    // verb form — a token that cleans to nothing
    // standing inside the phrase ("printed Sunday
    // \u{200b} bulletins") is nothing in it.
    let next1 = next_nonempty(segment, off + 1);
    let next2 = next1.and_then(|n| next_nonempty(segment, n + 1));
    let heads_noun_phrase = next1.is_some_and(|n| noun_phrase_word(&segment[n]))
        || (next1.is_some_and(|n| weekday_word(&segment[n]).is_some())
            && next2.is_some_and(|n| noun_phrase_word(&segment[n])));
    // Round 22 (R22-6): the determiner shield
    // spans its noun phrase — an irregular
    // PARTICIPLE form heading a noun phrase is
    // shielded by a determiner (or possessive)
    // standing ANYWHERE before it in the segment,
    // the R19-6 active-"for" rule applied to the
    // reach's shield: "the big left luggage" and
    // "the big sung hymns" are noun phrases exactly
    // as "the left luggage" is, the intervening
    // adjective being the phrase's own material.
    // Before this round the shield covered only
    // the immediately preceding word, and the
    // irregular branch below returned finite
    // before the heads-a-noun-phrase test was ever
    // reached, so one stacked adjective killed the
    // claim. The span is scoped to the forms with
    // a participle use — the branch whose
    // unconditional return was the defect. A
    // past-tense-only irregular ("sang", "rang",
    // "took") has no adjective use in any
    // position: "the team from Kent sang hymns"
    // stays a clause by the subject rule below,
    // its subject's determiner notwithstanding,
    // exactly as a regular form with a subject
    // head before it ("the choir folded
    // bulletins") does. An unshielded participle
    // ("sung hymns", no determiner phrase before
    // it) still ends the reach by the branch
    // below, and a verb form that heads no noun
    // phrase ("the choir sang") is judged by the
    // subject rule as always.
    if heads_noun_phrase
        && IRREGULAR_FINITE_FORMS.contains(&segment[off].as_str())
        && !IRREGULAR_PAST_ONLY_FORMS.contains(&segment[off].as_str())
        && segment[..off].iter().any(|prev| {
            matches!(prev.as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&prev.as_str())
        })
    {
        return false;
    }
    if segment_subject_before(segment, off) {
        return true;
    }
    // Round 23 (R23-6): a verb-form word standing
    // directly after a weekday word and heading a
    // noun phrase is the weekday-modifier phrase's
    // adjective, not a finite verb — "Sunday left
    // luggage", "Sunday taken seats" in an object
    // continuation are the phrase's own material,
    // and the irregular branch below must not make
    // a clause of them. The one exception is a
    // weekday standing as a subject PP's object:
    // in "the choir on Sunday sang hymns" the word
    // after the weekday is the clause's verb, and
    // it stays finite by the subject rule and this
    // branch alike (the preposition — or its
    // determiner — stands directly before the
    // weekday there).
    if heads_noun_phrase
        && off > 0
        && weekday_word(&segment[off - 1]).is_some()
        && !(off >= 2 && is_subject_preposition(segment[off - 2].as_str()))
        && !(off >= 3
            && matches!(segment[off - 2].as_str(), "the" | "a" | "an")
            && is_subject_preposition(segment[off - 3].as_str()))
    {
        return false;
    }
    if IRREGULAR_FINITE_FORMS.contains(&segment[off].as_str()) {
        return true;
    }
    if heads_noun_phrase {
        return false;
    }
    if off + 1 == segment.len() && segment[..off].iter().any(|w| noun_phrase_word(w)) {
        return false;
    }
    true
}

/// Whether `word` can stand as a noun-like word in
/// its segment (round 19, R19-4): a content word —
/// not a determiner, conjunction, preposition,
/// possessive, auxiliary, weekday, governor, or
/// tolerated adverb, and not an irregular finite
/// form. This is the word class that can head the
/// noun phrase an -ed adjective modifies, and the
/// word class that can stand as a clause's subject.
pub(super) fn noun_like_word(word: &str) -> bool {
    !word.is_empty()
        && !matches!(
            word,
            "the"
                | "a"
                | "an"
                | "and"
                | "or"
                | "but"
                | "then"
                | "to"
                | "for"
                | "from"
                | "of"
                | "on"
                | "in"
                | "at"
                | "by"
        )
        && !AUXILIARY_WORDS.contains(&word)
        && !POSSESSIVE_DETERMINERS.contains(&word)
        && weekday_word(word).is_none()
        && !direct_single_governor(word)
        && !is_tolerated_adverb(word)
        && !IRREGULAR_FINITE_FORMS.contains(&word)
}

/// Whether a run of words holds a finite verb form
/// (round 18, R18-8): the test `target_claim`'s
/// post-date reach and the active-"for" object scan
/// share. A verb form standing directly after a
/// determiner or a possessive is that noun phrase's
/// own material, not its verb — a finite verb never
/// stands immediately after a determiner in a
/// clause; it stands after the subject's head noun.
/// So "the meeting" and "the baked beans" hold no
/// verb, while "choir sang" and "the choir sang"
/// do (the verb follows the head noun, not the
/// determiner).
///
/// Round 19 (R19-4) tempered only generic -ed
/// forms this way; round 20 (R20-4) makes the
/// test one for every verb form (see
/// `verb_form_is_finite`): listed stems,
/// irregulars, and -ing forms in the adjective
/// slot ("planned dinner", "planning meeting")
/// read as modifiers exactly as "baked beans"
/// does, a real clause ("choir folded
/// bulletins") ends the reach exactly as "choir
/// sang" does, and a postposed form ("dinner
/// rolls, baked") continues it. Round 21 (R21-6)
/// states the test as v2 (see
/// `verb_form_is_finite`): the subject is a run
/// of words, an irregular form is finite wherever
/// it stands, and an auxiliary heading its own
/// clause ("choir was singing") is a finite verb
/// too (see `auxiliary_heads_clause`).
/// Whether the auxiliary at `off` in its segment
/// heads a finite clause of its own (round 21,
/// R21-6): a subject stands before it in its
/// segment (the run rule of
/// `segment_subject_before`) and an ordinary
/// verb form stands after it as its complement,
/// across any tolerated adverbs — "choir was
/// singing", "choir had sung". Such an auxiliary
/// is itself the clause's finite verb: a
/// progressive or perfect second clause is a
/// clause, and its auxiliary is not a bare
/// marker word standing in front of the date the
/// clause names.
/// The complement verb form of the auxiliary at
/// `off`, if one stands there (round 21, R21-6;
/// round 22, R22-4): the verb form the auxiliary
/// carries, across the adverbs that may stand
/// between them. The claim's own passive
/// constructions keep their standing readings —
/// this is clause DETECTION only, not the shared
/// adverb walk's spending rule: the tolerated
/// adverbs cross as they always have, and ONE
/// further adverb of any kind (an unlisted -ly
/// adverb, "not", "never") may stand in the slot
/// too — "choir was loudly singing" is a finite
/// clause exactly as "choir was happily singing"
/// is. Two unlisted adverbs, or any other word,
/// end the scan. The complement must be an
/// ordinary verb form, not a governor participle:
/// an auxiliary whose participle is itself a
/// governor ("it was moved", "it was cancelled")
/// is the governor machinery's own construction,
/// spent or unspent by the spent-predicate rule —
/// the auxiliary keeps its standing there and
/// this rule does not reach it.
///
/// Round 23 (R23-4): the auxiliary's slot holds
/// TWO words, not one — each a negation or an
/// adverb, listed or not. The two tolerances
/// R22-4 pinned separately compose: a negation
/// AND an adverb ("was not loudly played", "was
/// not ever played", "was really not played",
/// "was not yet played"), and two adverbs ("was
/// softly gently singing", "was loudly
/// beautifully played"), read as one auxiliary
/// complement exactly as either tolerance alone
/// did. "ever" and "yet" are the slot's non--ly
/// adverbs the witnesses wear; a third slot word
/// ends the scan, as any other word does.
pub(super) fn aux_complement(segment: &[String], off: usize) -> Option<usize> {
    let mut j = off + 1;
    let mut unlisted_used = 0usize;
    while let Some(w) = segment.get(j) {
        let w = w.as_str();
        if w.is_empty() {
            // Round 27 (R27-A, F3): nothing in the
            // slot is crossed as nothing, uncounted.
            j += 1;
        } else if is_tolerated_adverb(w) {
            j += 1;
        } else if adverbial_word(w) || matches!(w, "not" | "never" | "ever" | "yet" | "so" | "too")
        {
            if unlisted_used >= 2 {
                break;
            }
            unlisted_used += 1;
            j += 1;
        } else {
            break;
        }
    }
    match segment.get(j) {
        Some(w) if is_finite_verb_form(w) && !is_single_governor(w) => Some(j),
        _ => None,
    }
}

pub(super) fn auxiliary_heads_clause(segment: &[String], off: usize) -> bool {
    if !AUXILIARY_WORDS.contains(&segment[off].as_str()) {
        return false;
    }
    if !segment_subject_before(segment, off) {
        return false;
    }
    aux_complement(segment, off).is_some()
}

/// Whether the auxiliary at `off` heads a finite
/// clause with NO subject before it in its
/// segment (round 22, R22-4): a subjectless
/// auxiliary + participle construction — "then
/// was played", "then was singing" — is a second
/// clause whose subject the reader supplies from
/// the claim's own clause, exactly as its "were"
/// twin has always read. Before this round bare
/// "was" alone of the auxiliaries escaped the
/// reading: it is itself a `direct_single_governor`
/// (the R12 past-marker family), so the claim
/// died on the between-words fence AND the target
/// date stood governed by the "was" of its own
/// clause — total silence. A bare "was" with no
/// participle after it ("(was November 1)") keeps
/// governing as the past marker it is: this rule
/// needs the complement the clause is built from.
pub(super) fn auxiliary_subjectless_clause(segment: &[String], off: usize) -> bool {
    if !AUXILIARY_WORDS.contains(&segment[off].as_str()) {
        return false;
    }
    if segment_subject_before(segment, off) {
        return false;
    }
    aux_complement(segment, off).is_some()
}

pub(super) fn segment_has_finite_verb(segment: &[String]) -> bool {
    segment.iter().enumerate().any(|(off, w)| {
        (is_finite_verb_form(w) && verb_form_is_finite(segment, off))
            || auxiliary_heads_clause(segment, off)
            // Round 22 (R22-4): a subjectless
            // auxiliary + participle construction
            // is a finite clause for the reach's
            // test too ("then was played hymns" —
            // the participle reads as a modifier
            // standing alone, but the construction
            // is a clause).
            || auxiliary_subjectless_clause(segment, off)
    })
}

/// Whether an active-"for" object phrase holds a
/// finite verb form (round 19, R19-6): the shared
/// segment test, with the determiner shield widened
/// for this scan only — a determiner (or possessive)
/// standing ANYWHERE before the verb-form word in
/// the object protects it, because the word is the
/// object phrase's head and the words between the
/// determiner and the head are the phrase's own
/// modifiers: "the big meeting", "the church
/// meeting", "the Johnson meeting", "a big meeting"
/// hold no verb, exactly as bare "the meeting" does.
/// The claim reach keeps the immediate-determiner
/// rule (see `segment_has_finite_verb`): there, "the
/// choir sang" is a clause, not a noun phrase.
pub(super) fn active_for_object_has_finite_verb(object: &[String]) -> bool {
    object.iter().enumerate().any(|(off, w)| {
        if !is_finite_verb_form(w) {
            return false;
        }
        if object[..off].iter().any(|prev| {
            matches!(prev.as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&prev.as_str())
        }) {
            return false;
        }
        // The rest of the shared test, judged on the
        // word's own segment position: the one verb
        // test of round 20 (R20-4) — a verb form is
        // finite only with a noun-like subject
        // standing before it in the object — which
        // subsumes the generic -ed adjective and
        // subject rules of R19-4.
        verb_form_is_finite(object, off)
    })
}

/// The backward walk over the passive-"for"
/// construction's own complement — the slot between
/// its auxiliary and its participle (round 24,
/// R24-6): the walk the passive-for scan uses, and
/// no other scan. Detection (`aux_complement`)
/// crosses an unlisted adverb in this slot on its
/// own — one unlisted adverb, or two, spend exactly
/// as a listed pair does — while the shared
/// spending walk (`aux_adverb_walk_bounded`)
/// crossed an unlisted adverb only beside a
/// negation, so "(it was softly gently moved for
/// November 1)" detected as a clause on the claim
/// surface and spent nothing on its own surface,
/// and "(it was loudly moved for November 1)" spent
/// nothing either. This walk aligns the passive-for
/// complement with detection: the slot holds up to
/// TWO slot words — a tolerated adverb, an
/// adverbial word of any kind, "ever" / "yet", or a
/// negation (a negation counts as one slot word,
/// as the R23-4 rule states) — in the post- and
/// pre-auxiliary positions alike, and the fronted
/// stack standing before the subject is judged by
/// `fronted_stack_ok` under the same cap, exactly
/// as the shared walk judges it. The governor
/// scans keep the shared walk and its caps
/// unchanged: they guard a governor's reach across
/// dates, not a local complement inside one
/// construction, and a three-word slot still does
/// not spend there.
///
/// Round 25 (R25-B, M9 = red team RT25-9): "so" and
/// "too" join the slot vocabulary as intensifiers
/// in BOTH slot walks — this walk and detection's
/// `aux_complement` — keeping the scans aligned:
/// "(it was so gently moved for November 1)" and
/// "(it was too loudly moved for November 1)" spend
/// exactly as their adverb twins do. The phrasal
/// adverbials ("at all" / "at once" / "in all") are
/// recorded, not fixed: they would need unit slots,
/// and "not at all" exceeds the pinned two-word
/// cap, which stands.
pub(super) fn passive_for_slot_walk(tokens: &[String], start: usize) -> Option<(usize, usize)> {
    let mut j = start;
    let mut aux_seen = false;
    let mut adverbs_used = 0usize;
    while j > 0 {
        let prev = tokens[j - 1].as_str();
        if prev.is_empty() {
            // Round 27 (R27-A, F3 = review R27-4):
            // a token that cleans to nothing in the
            // slot ("it was \u{200b} moved for
            // November 1") is nothing — crossed
            // without counting toward the cap, as
            // the claim scan's sites already cross
            // it.
            j -= 1;
            continue;
        }
        if AUXILIARY_WORDS.contains(&prev) {
            aux_seen = true;
            j -= 1;
        } else if (matches!(prev, "not" | "never") && aux_beyond_slot(tokens, j - 1))
            || is_tolerated_adverb(prev)
            || adverbial_word(prev)
            || matches!(prev, "ever" | "yet" | "so" | "too")
        {
            // One slot word, of whichever kind: a
            // negation (its auxiliary beyond it, as
            // the shared walk requires), a tolerated
            // adverb, an adverbial word of any kind,
            // or "ever" / "yet" — each counts once
            // toward the two-word cap.
            if adverbs_used >= 2 {
                return None;
            }
            adverbs_used += 1;
            j -= 1;
        } else {
            break;
        }
    }
    if !aux_seen {
        return None;
    }
    if j > 0 && !fronted_stack_ok(tokens, j, adverbs_used) {
        return None;
    }
    Some((j, adverbs_used))
}

pub(super) fn passive_for_zone_start(before: &[String], window: &[String]) -> Option<usize> {
    // Round 27 (R27-A, F3): the construction's
    // "for" is the window's last word that is
    // something — a token that cleans to nothing
    // standing between it and the date is nothing.
    let for_pos = window.iter().rposition(|w| !w.is_empty())?;
    if window.len() < 3 || window[for_pos].as_str() != "for" {
        return None;
    }
    // The participle phrase ends just before "for":
    // a phrasal pair ("put back"), or one word.
    // Round 27 (R27-A, F3): tokens that clean to
    // nothing between the participle and the "for"
    // ("it was moved \u{200b} for November 1") are
    // nothing — the participle is the nearest word
    // that is something before the "for", and the
    // phrasal pair is read over the two nearest.
    let mut prev_words: Vec<usize> = Vec::new();
    let mut scan = for_pos;
    while scan > 0 && prev_words.len() < 2 {
        scan -= 1;
        if !window[scan].is_empty() {
            prev_words.push(scan);
        }
    }
    let participle_start = if prev_words.len() == 2
        && is_phrasal_governor_pair(&window[prev_words[1]], &window[prev_words[0]])
    {
        prev_words[1]
    } else if let Some(&pp) = prev_words.first() {
        let word = window[pp].as_str();
        let is_participle = is_cancellation_verb(word) || word.ends_with("ed") || word == "held";
        if !is_participle {
            return None;
        }
        pp
    } else {
        return None;
    };
    // Round 18 (R18-10): the tolerance is up to TWO
    // consecutive listed adverbs — "(it was just
    // recently moved for November 1)" spends like
    // its one-adverb twins. Three stacked still
    // break the scan, as any unlisted word does.
    // Round 20 (R20-5): the walk is the one shared
    // walk (`aux_adverb_walk`) — the same list and
    // the same cap in the post-auxiliary,
    // pre-auxiliary, and fronted positions.
    // Round 24 (R24-6): the passive-for scan's own
    // complement walk is `passive_for_slot_walk` —
    // it crosses adverbs as detection does (an
    // unlisted adverb needs no negation beside it),
    // under the same two-slot-word cap; the shared
    // walk and the governor scans it serves are
    // unchanged.
    let (j, adverbs_used) = passive_for_slot_walk(window, participle_start)?;
    // A fronted stack standing in its own clause
    // before the window (the window starts after
    // its clause break) is judged by the same rule:
    // the adverbial run at the end of `before`
    // must be listed throughout and fit the cap
    // together with the walk's own adverbs.
    let mut run = 0usize;
    let mut run_all_listed = true;
    for w in before.iter().rev() {
        if adverbial_word(w) {
            run += 1;
            if !is_tolerated_adverb(w) {
                run_all_listed = false;
            }
        } else {
            break;
        }
    }
    if run > 0 && (!run_all_listed || run + adverbs_used > 2) {
        return None;
    }
    Some(j)
}

/// The start of an ACTIVE "for" construction
/// standing at the end of a governor window, if one
/// does (round 18, R18-9): the window ends in "for",
/// and a displacement phrase — a displacement verb
/// in any form, or a "put back" / "push back" pair,
/// the same family the passive rule is scoped by —
/// stands before the "for" with nothing between it
/// and the "for" but the phrase's own object: object
/// words only, no second verb, no governor, no
/// auxiliary, no weekday. "(we moved it for
/// November 1)" names November 1 as the occasion in
/// the active voice, as "(it was moved for
/// November 1)" does in the passive: the verb is
/// spent on its object, and "for" names the date.
/// The scope is the displacement family alone, as
/// the disposition states — an active cancellation
/// ("we cancelled it for November 1") keeps
/// governing its date, and the bare parity "(we
/// moved it November 1)" stands: with no "for"
/// there is no construction here. Round 19
/// (R19-6): the object's no-second-verb test is
/// `active_for_object_has_finite_verb`, whose
/// determiner shield reaches across the object's
/// modifiers.
pub(super) fn active_for_zone_start(window: &[String]) -> Option<usize> {
    let for_pos = window.iter().rposition(|w| !w.is_empty())?;
    if window.len() < 2 || window[for_pos].as_str() != "for" {
        return None;
    }
    // The nearest displacement phrase ending at or
    // before the "for": try the phrase that ends
    // closest to the "for" first, so an object that
    // happens to hold a verb form of its own ("we
    // moved the move for …") is read by the nearest
    // phrase, and a phrase too far back — behind a
    // second verb — never reaches across it.
    // Round 28 (R28-A, F2): the phrase is read over
    // the window's WORDS (see `word_phrase`), so a
    // pair wearing a nothing between its words
    // ("put \u{200b} back for …") is the pair; the
    // object slice and the returned start stay in
    // token positions, as the callers judge them.
    let word_pos: Vec<usize> = (0..for_pos).filter(|&p| !window[p].is_empty()).collect();
    for e in (0..word_pos.len()).rev() {
        for len in 1..=2usize {
            if e + 1 < len {
                continue;
            }
            let phrase: Vec<String> = word_pos[e + 1 - len..=e]
                .iter()
                .map(|&p| window[p].clone())
                .collect();
            if !is_displacement_phrase(&phrase) {
                continue;
            }
            let end = word_pos[e] + 1;
            let object = &window[end..for_pos];
            let object_ok = !active_for_object_has_finite_verb(object)
                && object.iter().all(|w| {
                    // Round 27 (R27-A, F3 = review
                    // R27-4): an object word that
                    // cleans to nothing ("moved
                    // \u{200b} for November 1",
                    // "moved it \u{200b} for
                    // November 1") is nothing in the
                    // object — it passes as the
                    // object's other words pass,
                    // judged on the words that are
                    // something.
                    w.is_empty()
                        || (!direct_single_governor(w)
                            && !is_cancellation_verb(w)
                            && !AUXILIARY_WORDS.contains(&w.as_str())
                            && weekday_word(w).is_none()
                            && w.as_str() != "to"
                            && w.as_str() != "for")
                });
            if object_ok {
                return Some(word_pos[e + 1 - len]);
            }
        }
    }
    None
}

pub(super) fn governed_before(before: &[String], window: &[String]) -> bool {
    // Round 17 (R17-4): governor phrases standing
    // inside a passive "for" construction at the
    // window's end — the auxiliary and the participle
    // themselves — are the construction, not
    // governors of the date the "for" names. A
    // governor standing before the construction
    // still governs.
    // Round 18 (R18-9): the active "for" construction
    // is a zone the same way — the displacement verb
    // and its object are the construction, not
    // governors of the occasion date. The zone that
    // starts earlier covers both.
    let mut for_zone_start = passive_for_zone_start(before, window);
    if let Some(active_start) = active_for_zone_start(window) {
        for_zone_start = Some(match for_zone_start {
            Some(passive_start) => passive_start.min(active_start),
            None => active_start,
        });
    }
    for i in 0..window.len() {
        // Round 31 (R31-C, F4): the phrase-start
        // gate, as in `governed_after` — a word that
        // begins no governor phrase is judged by
        // `phrase_governs` no further.
        if !governor_phrase_may_start(&window[i]) {
            continue;
        }
        // Round 27 (R27-A, F3): a phrase never
        // STARTS on a token that cleans to nothing
        // — the phrase matchers read through an
        // interior nothing, but the phrase's start
        // is its first word that is something, and
        // the scans judge positions (the for-zone,
        // the "to" adjacency) by that start.
        if window[i].is_empty() {
            continue;
        }
        for len in 1..=3usize {
            // Round 28 (R28-A, F2): the phrase is
            // `len` WORDS (see `word_phrase`), and
            // the "to" it would stand before is the
            // next word that is something after the
            // phrase's own last word.
            let Some((phrase, phrase_end)) = word_phrase(window, i, len) else {
                break;
            };
            if !phrase_governs(window, i, &phrase) {
                continue;
            }
            if let Some(zone_start) = for_zone_start {
                if i >= zone_start {
                    continue;
                }
            }
            // Round 15 (R15-1): a displacement
            // phrase standing directly before a
            // "to" governs the date it moves FROM;
            // the date the "to" names is its
            // target and asserts — "(rescheduled
            // to November 8)" and "(moved to
            // November 8)" name November 8, the
            // held "(moved to ...)" shape, and in
            // "(November 1, moved to November 8)"
            // the phrase governs November 1
            // through `governed_after` instead.
            if next_nonempty(window, phrase_end).is_some_and(|n| window[n].as_str() == "to")
                && is_displacement_phrase(&phrase)
            {
                continue;
            }
            return true;
        }
    }
    false
}

/// The possessive determiners (round 14, R14-5).
pub(super) const POSSESSIVE_DETERMINERS: [&str; 7] =
    ["his", "her", "their", "my", "our", "your", "its"];

/// The subject pronouns (round 27, R27-A, F1
/// mechanism 5): the pronoun forms that head a
/// clause — "they / we / she / he / it / you / i".
/// The object forms (them / us / him) are not
/// here: they cannot head the conjoined clause
/// the boundary arms on.
pub(super) const SUBJECT_PRONOUNS: [&str; 7] = ["they", "we", "she", "he", "it", "you", "i"];

/// Whether the phrase standing at `i` in `slice`
/// governs, in its context (round 11–14): the
/// families of `is_governor_phrase`, with one
/// contextual narrowing — round 14 (R14-5), a bare
/// "wont" standing immediately after a possessive
/// determiner is the noun "wont" (accustomed), not
/// the unpunctuated "won't", so "(his wont,
/// November 1)" names its date. The contraction
/// "won't" itself is unaffected.
/// Whether `word`, standing at `i` in `slice`, is an
/// -s form of a cancellation stem heading a
/// determiner-headed noun phrase (round 16, RT16-6):
/// the word itself is an -s inflection (not the bare
/// stem), and walking back over noun-like words —
/// words that are not verb forms, governors,
/// auxiliaries, or weekdays — reaches a determiner
/// ("the", "a", "an", "these", "those") within three
/// words. "The drops" and "the cough drops" are noun
/// phrases: a finite verb cannot stand as the head of
/// one, so the -s word does not govern in that slot,
/// exactly as the possessive-preceded "wont" (round
/// 14, R14-5) is the noun after its determiner. Bare
/// -s forms ("choir cancels", "ticket drops" as an
/// event label) keep governing: no determiner heads
/// them.
pub(super) fn determiner_headed_s_form(slice: &[String], i: usize, word: &str) -> bool {
    let is_s_form = CANCELLATION_STEMS.iter().any(|stem| {
        word.strip_suffix('s') == Some(*stem) || word.strip_suffix("es") == Some(*stem)
    });
    if !is_s_form || i == 0 {
        return false;
    }
    let mut j = i;
    let mut steps = 0;
    while j > 0 && steps < 3 {
        let prev = slice[j - 1].as_str();
        if matches!(prev, "the" | "a" | "an" | "these" | "those") {
            return true;
        }
        // Only a noun-like word may stand between the
        // determiner and its head; anything else ends
        // the phrase.
        if direct_single_governor(prev)
            || is_cancellation_verb(prev)
            || AUXILIARY_WORDS.contains(&prev)
            || weekday_word(prev).is_some()
        {
            return false;
        }
        j -= 1;
        steps += 1;
    }
    false
}

/// Whether the "from" at `i` in `slice` stands
/// inside a subject prepositional phrase (round
/// 22, R22-5): a subject head run before it ("the
/// team from …"), its own noun-like object run
/// after it ("… Kent"), and the clause's finite
/// verb after that ("… sang"). Such a "from"
/// governs nothing — it is the subject phrase's
/// own preposition, not the origin marker of the
/// "(from November 1)" family — but its clause
/// stands in the target date's governor window,
/// and before this round the window's scan could
/// not tell a subject's "from Kent" from a
/// governing "from", so "the team from Kent sang"
/// silenced both dates for every verb form. A
/// "from" with no subject head before it, or no
/// clause verb after its object, governs as it
/// always has.
pub(super) fn from_stands_in_subject_pp(slice: &[String], i: usize) -> bool {
    if !subject_head_before(slice, i) {
        return false;
    }
    // The object itself: determiners, then a
    // weekday object or a noun-like run (a framed
    // date's words included).
    let mut k = i + 1;
    while slice.get(k).is_some_and(|w| {
        matches!(w.as_str(), "the" | "a" | "an") || POSSESSIVE_DETERMINERS.contains(&w.as_str())
    }) {
        k += 1;
    }
    let object_start = k;
    if slice.get(k).is_some_and(|w| weekday_word(w).is_some()) {
        k += 1;
    } else {
        while slice
            .get(k)
            .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
        {
            k += 1;
        }
    }
    if k == object_start {
        return false;
    }
    // Round 23 (R23-5): the clause's verb may
    // stand past a SECOND prepositional phrase —
    // "the team from Kent on Sunday sang" — the
    // shared forward walk crosses the whole stack
    // before judging the verb.
    verb_after_pp_objects(slice, k)
}

/// Whether the negation at `i` in `slice` stands
/// in an auxiliary's adverb slot (round 22, R22-4;
/// round 23, R23-4): scanning back from the
/// negation crosses only slot words — tolerated
/// adverbs, adverbs of any kind, "ever" / "yet",
/// and further negations — and reaches an
/// auxiliary whose complement (the `aux_complement`
/// scan) stands AFTER the negation. R22-4's rule
/// reached only a negation standing directly
/// after its auxiliary ("was not played"); the
/// two-word slot puts the negation in either
/// position ("was really not played", "was loudly
/// never played", "was ever not played"), and in
/// every position the negation is spent on its
/// verb, not on the date its clause names. A
/// negation with no such construction around it
/// ("was not November 8", "(not November 8)")
/// governs as always.
pub(super) fn negation_in_aux_slot(slice: &[String], i: usize) -> bool {
    let mut j = i;
    while j > 0 {
        let w = slice[j - 1].as_str();
        if AUXILIARY_WORDS.contains(&w) {
            return match aux_complement(slice, j - 1) {
                Some(comp) => comp > i,
                None => false,
            };
        }
        if is_tolerated_adverb(w)
            || adverbial_word(w)
            || matches!(w, "not" | "never" | "ever" | "yet")
        {
            j -= 1;
        } else {
            return false;
        }
    }
    false
}

pub(super) fn phrase_governs(slice: &[String], i: usize, phrase: &[String]) -> bool {
    // Round 28 (R28-A, F2): the phrase arrives as
    // WORDS (see `word_phrase`), so its length is
    // its word count — the one-word narrowings
    // below judge a one-word phrase, never a
    // one-word phrase wearing a nothing-token in a
    // two-position slice.
    let len = phrase.len();
    if !is_governor_phrase(phrase) {
        return false;
    }
    // Round 22 (R22-5): a "from" standing inside a
    // subject prepositional phrase is that phrase's
    // own word, not an origin governor (see
    // `from_stands_in_subject_pp`).
    if len == 1 && phrase[0] == "from" && from_stands_in_subject_pp(slice, i) {
        return false;
    }
    // Round 16 (RT16-6): a determiner-headed -s word
    // is a noun phrase's head, not a verb.
    if len == 1 && determiner_headed_s_form(slice, i, &phrase[0]) {
        return false;
    }
    // Round 21 (R21-6): an auxiliary standing as
    // the finite verb of its own clause — a
    // subject before it in the slice and a verb
    // form after it as its complement ("choir was
    // singing") — is that clause's verb, not the
    // bare past marker: the "was" of "choir was
    // singing to November 8" does not govern the
    // date its own clause names, exactly as the
    // "was" of "(choir was singing, November 1)"
    // never governed across its clause break. A
    // bare "was" with no clause around it ("(was
    // November 1)") keeps governing as the past
    // marker it is, and a governor participle
    // standing after the auxiliary ("it was
    // moved") governs by its own phrase below.
    // Round 22 (R22-4): the same holds for a
    // SUBJECTLESS auxiliary + participle
    // construction standing in the window — "then
    // was played to November 8" names November 8
    // in a second clause the reader completes,
    // and its "was" does not govern the date its
    // own clause names either; November 8 stands
    // on its own standing there.
    if len == 1
        && phrase[0] == "was"
        && (auxiliary_heads_clause(slice, i) || auxiliary_subjectless_clause(slice, i))
    {
        return false;
    }
    // Round 22 (R22-4): a negation standing in an
    // auxiliary's own adverb slot — "then was not
    // played", "then was never played" — negates
    // the clause's verb, not the date the clause
    // names, and does not govern past it. Round 23
    // (R23-4): the slot holds two words, so the
    // negation may stand in either position in it
    // (see `negation_in_aux_slot`).
    if len == 1 && matches!(phrase[0].as_str(), "not" | "never") && negation_in_aux_slot(slice, i) {
        return false;
    }
    if len == 1
        && (phrase[0] == "wont" || phrase[0].starts_with("wont-"))
        && i > 0
        && POSSESSIVE_DETERMINERS.contains(&slice[i - 1].as_str())
    {
        return false;
    }
    true
}

/// The cancellation verb stems (round 14, R14-1):
/// the verbs whose inflections govern by stem, the
/// way round 13's contraction class governs by
/// suffix. Noun derivations are not forms of the
/// verb: "cancellation" and "postponement" govern
/// nothing (pinned in the r14a tests).
pub(super) const CANCELLATION_STEMS: [&str; 13] = [
    "cancel",
    "scratch",
    "drop",
    "postpone",
    "remove",
    "delay",
    "omit",
    "disregard",
    "forget",
    // Round 15 (R15-1): the displacement verbs
    // govern like the cancellation verbs — a date
    // a sender moved, shifted, bumped, or
    // rescheduled is precisely the date that is
    // not it, and with no new date named there is
    // nothing to lose by silencing it. Where a
    // new date IS named through "to", the target
    // asserts (see `governed_before`).
    "move",
    "shift",
    "bump",
    "reschedule",
];

/// The displacement verb stems (round 15,
/// R15-1): the subset of the cancellation stems
/// that name a move to another date rather than
/// an end. A displacement phrase standing before
/// a "to" governs the date it moves FROM, never
/// the target the "to" names.
pub(super) const DISPLACEMENT_STEMS: [&str; 4] = ["move", "shift", "bump", "reschedule"];

/// Whether `word` is a form of one of the
/// displacement verbs (round 15, R15-1).
pub(super) fn is_displacement_verb(word: &str) -> bool {
    DISPLACEMENT_STEMS
        .iter()
        .any(|stem| is_verb_form(word, stem))
}

/// Whether a two-word phrase is one of the
/// phrasal governors whose first word inflects
/// (round 15, R15-1): "call off" in any of
/// call's forms (called off, calling off, calls
/// off), "put back" and "push back" in any of
/// their forms (pushed back, putting back), the
/// older "put off" in put's forms, and "forget
/// about" in any of forget's forms, the
/// irregular "forgot about" included. A bare
/// "call", "push", or "put" with no second word
/// governs nothing, as a bare "put" never has.
pub(super) fn is_phrasal_governor_pair(a: &str, b: &str) -> bool {
    (is_verb_form(a, "call") && b == "off")
        || (is_verb_form(a, "put") && matches!(b, "off" | "back"))
        || (is_verb_form(a, "push") && b == "back")
        || (is_verb_form(a, "forget") && b == "about")
}

/// Whether one run of words is a displacement
/// phrase (round 15, R15-1): a displacement verb
/// on its own, or a "put back" / "push back"
/// pair. These are the phrases a "to" after them
/// turns into a from/to shape.
pub(super) fn is_displacement_phrase(phrase: &[String]) -> bool {
    let words: Vec<&str> = phrase
        .iter()
        .map(String::as_str)
        .filter(|w| !w.is_empty())
        .collect();
    match words.as_slice() {
        [w] => is_displacement_verb(w),
        [a, b] => (is_verb_form(a, "put") || is_verb_form(a, "push")) && *b == "back",
        _ => false,
    }
}

/// A stem with a doubled final consonant undoubled:
/// "dropp" -> "drop", "cancell" -> "cancel".
/// Anything else is returned unchanged.
pub(super) fn undoubled(stem: &str) -> &str {
    let bytes = stem.as_bytes();
    if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
        &stem[..stem.len() - 1]
    } else {
        stem
    }
}

/// Whether `word` is an inflection of the verb
/// `stem`: the base form, an -ed form, an -ing
/// form, or an -s form, with the spelling adjustments English
/// makes — a final "e" drops before -ing
/// (postponing, removing) and takes a bare "d" for
/// the past (postponed, removed), and a final
/// consonant may double (dropping, omitting, and
/// "cancel"'s British cancelling / cancelled).
/// Round 15 (R15-2): the -s form (cancels,
/// postpones, scratches — the last with the -es
/// spelling a sibilant-final stem takes) is a form
/// of the verb like any other, and "forget" has an
/// irregular past the suffix rules cannot build:
/// "forgot" and "forgotten" are its forms too.
pub(super) fn is_verb_form(word: &str, stem: &str) -> bool {
    if word == stem {
        return true;
    }
    if stem == "forget" && matches!(word, "forgot" | "forgotten") {
        return true;
    }
    if let Some(base) = word.strip_suffix('s') {
        if base == stem {
            return true;
        }
    }
    if let Some(base) = word.strip_suffix("es") {
        if base == stem {
            return true;
        }
    }
    if let Some(base) = word.strip_suffix("ing") {
        if base == stem || undoubled(base) == stem {
            return true;
        }
        if stem.ends_with('e') && &stem[..stem.len() - 1] == base {
            return true;
        }
    }
    if let Some(base) = word.strip_suffix("ed") {
        if base == stem || undoubled(base) == stem {
            return true;
        }
    }
    if stem.ends_with('e') && word.strip_suffix('d') == Some(stem) {
        return true;
    }
    false
}

/// Whether `word` is a form of one of the
/// cancellation verbs (round 14, R14-1).
pub(super) fn is_cancellation_verb(word: &str) -> bool {
    CANCELLATION_STEMS
        .iter()
        .any(|stem| is_verb_form(word, stem))
}

/// Whether one token is a governor on its own
/// (round 11–14): the single words of the families,
/// stated once. Round 14 (R14-4): a hyphenated
/// compound governs when a part of it governs or
/// its parts form a governor phrase — "do-not-use",
/// "no-longer", "hold-off" — the hyphens split for
/// this test only; the token is read whole
/// everywhere else.
pub(super) fn is_single_governor(word: &str) -> bool {
    if direct_single_governor(word) {
        return true;
    }
    if !word.contains('-') {
        return false;
    }
    let parts: Vec<String> = word
        .split('-')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if parts.len() < 2 {
        return false;
    }
    for i in 0..parts.len() {
        for len in 1..=3usize {
            let Some(phrase) = parts.get(i..i + len) else {
                break;
            };
            if is_governor_phrase(phrase) {
                return true;
            }
        }
    }
    false
}

/// Whether one token, read whole, is a governor on
/// its own (round 11–14): the contraction class,
/// the negation and origin singles, "nope" (round
/// 14, R14-3), and the cancellation verbs by stem
/// (round 14, R14-1).
pub(super) fn direct_single_governor(w: &str) -> bool {
    w == "cannot"
        || w == "wont"
        || w.ends_with("n't")
        || w.ends_with("n\u{2019}t")
        || is_cancellation_verb(w)
        || matches!(
            w,
            "not"
                | "ignore"
                | "never"
                | "originally"
                | "from"
                | "skip"
                | "except"
                | "previously"
                | "formerly"
                | "was"
                | "no"
                | "nope"
                | "isnt"
                | "arent"
                | "wasnt"
                | "werent"
                | "wrong"
        )
}

/// Whether one run of words is a governor phrase
/// (round 11–13): the single words, the pairs, and
/// the triples the families recognize, stated once
/// so the window check and the clause-trailing
/// check (round 13, R13-2) read the same families.
/// Round 13 (R13-1): negation gains the contraction
/// class AS A CLASS — any token ending in "n't"
/// (either apostrophe), plus the unpunctuated
/// "wont" and "cannot", is a negation governor, the
/// rule a reader applies rather than another word
/// list; cancellation gains the verbs cancel /
/// scratch / drop / postpone / remove and the pair
/// "hold off"; the label word "wrong" (its colon
/// stripped by `group_token`) disowns its date.
pub(super) fn is_governor_phrase(phrase: &[String]) -> bool {
    // Round 27 (R27-A, F3): a phrase's words are
    // its words that are something — a token that
    // cleans to nothing standing inside a two-word
    // governor ("put \u{200b} back") does not
    // unmake the phrase, at this matcher or at the
    // cancellation / displacement / target
    // matchers below, which filter the same way.
    let words: Vec<&str> = phrase
        .iter()
        .map(String::as_str)
        .filter(|w| !w.is_empty())
        .collect();
    match words.as_slice() {
        [w] => is_single_governor(w),
        [a, b] => {
            is_phrasal_governor_pair(a, b)
                || matches!(
                    (*a, *b),
                    ("no", "longer")
                        | ("don't", "use")
                        | ("didn't", "use")
                        | ("dont", "use")
                        | ("didnt", "use")
                        | ("rather", "than")
                        | ("instead", "of")
                        | ("never", "mind")
                        | ("old", "date")
                        | ("hold", "off")
                        | ("put", "off")
                        | ("forget", "about")
                )
        }
        [a, b, c] => {
            matches!((*a, *b, *c), ("used", "to", "be") | ("do", "not", "use"))
        }
        _ => false,
    }
}

/// Whether the parts of a hyphenated token, read
/// as consecutive phrases, contain a cancellation
/// phrase (round 14, R14-2/R14-4): "hold-off"
/// standing after a date cancels it.
pub(super) fn hyphenated_cancellation(parts: &[String]) -> bool {
    for i in 0..parts.len() {
        for len in 1..=2usize {
            let Some(phrase) = parts.get(i..i + len) else {
                break;
            };
            if is_cancellation_phrase(phrase) {
                return true;
            }
        }
    }
    false
}

/// Whether one run of words is a cancellation
/// phrase (round 14, R14-2): a cancellation verb in
/// any of its stem forms, the phrasal pairs "hold
/// off" / "put off", "never mind", "forget about",
/// and a cancellation verb carrying its object
/// ("drop it", "scratch that", "cancel it"). These
/// are the forms that attach to a date they FOLLOW,
/// the way "cancelled" always has — the trailing
/// position draws from the same family definition
/// as the before position, so "(remove November 1)"
/// and "(November 1, removed)" disqualify alike.
pub(super) fn is_cancellation_phrase(phrase: &[String]) -> bool {
    let words: Vec<&str> = phrase
        .iter()
        .map(String::as_str)
        .filter(|w| !w.is_empty())
        .collect();
    match words.as_slice() {
        [w] => {
            is_cancellation_verb(w)
                || (w.contains('-') && {
                    let parts: Vec<String> = w
                        .split('-')
                        .filter(|p| !p.is_empty())
                        .map(str::to_string)
                        .collect();
                    parts.len() >= 2 && hyphenated_cancellation(&parts)
                })
        }
        [a, b] => {
            is_phrasal_governor_pair(a, b)
                || matches!(
                    (*a, *b),
                    ("hold", "off") | ("put", "off") | ("never", "mind") | ("forget", "about")
                )
                || (is_cancellation_verb(a) && matches!(*b, "it" | "that"))
        }
        _ => false,
    }
}

/// The auxiliary words (round 16, R16-3): auxiliaries
/// as a class, not a five-word list — the be-forms,
/// the have-forms, and the get-forms. A governor
/// participle standing as the predicate of one of
/// these is spent on its subject.
pub(super) const AUXILIARY_WORDS: [&str; 15] = [
    "am", "is", "are", "was", "were", "be", "been", "being", //
    "has", "had", "have", //
    "get", "gets", "got", "getting",
];

/// Whether a word may stand as an adverb of
/// tolerance between an auxiliary and its participle
/// (round 16, R16-3): one of the common short adverbs
/// — "it was just moved" is the witness shape.
/// Round 17 (R17-7): the tolerance set is a CLOSED
/// list — the short adverbs plus the attested -ly
/// adverbs named here. There is no -ly catch-all:
/// the catch-all admitted nouns and names standing
/// in the adverb slot ("it was family moved", "it
/// was kelly moved" spent like "recently"), and any
/// non-tolerated intervening word governs as it
/// always has outside this slot.
pub(super) fn is_tolerated_adverb(word: &str) -> bool {
    matches!(
        word,
        "just"
            | "already"
            | "still"
            | "also"
            | "even"
            | "now"
            | "soon"
            | "often"
            | "recently"
            | "previously"
            | "originally"
            | "finally"
            | "officially"
            | "tentatively"
            | "temporarily"
            | "currently"
            | "newly"
            | "jointly"
            // Round 18 (R18-10): then, briefly,
            // shortly, formally, unanimously, and
            // happily join the list — ordinary office
            // adverbs in exactly this slot, whose
            // absence silenced the occasion date.
            // Round 19 (R19-7): very, plain, and
            // simply join — the ordinary intensifiers
            // of "just plain moved" and "very recently
            // moved", whose absence broke a stack in
            // every scan.
            // Round 20 (R20-5): really, actually,
            // truly, quickly, definitely, probably,
            // and carefully join — the next tier of
            // ordinary office adverbs, whose absence
            // silenced even a target claim ("it was
            // really moved to November 8" read
            // nothing). The list is 34 words in all.
            | "then"
            | "briefly"
            | "shortly"
            | "formally"
            | "unanimously"
            | "happily"
            | "very"
            | "plain"
            | "simply"
            | "really"
            | "actually"
            | "truly"
            | "quickly"
            | "definitely"
            | "probably"
            | "carefully"
    )
}

/// Whether `word` is an adverbial word without
/// being on the tolerated list (round 20, R20-5):
/// an -ly word, or "yesterday". Used only to
/// recognise a fronted adverb STACK standing in
/// its own clause — a stack is a run of adverbial
/// words, listed or not; whether it tolerates is
/// judged on the list itself.
pub(super) fn adverbial_word(word: &str) -> bool {
    is_tolerated_adverb(word) || word == "yesterday" || word.ends_with("ly")
}

/// A run of words as a fronted adverb stack, if
/// it is one (round 20, R20-5): non-empty, and
/// every word adverbial. Returns the stack's word
/// count and whether every word is on the
/// tolerated list.
pub(super) fn adverb_stack_clause(words: &[String]) -> Option<(usize, bool)> {
    if words.is_empty() || !words.iter().all(|w| adverbial_word(w)) {
        return None;
    }
    let all_listed = words.iter().all(|w| is_tolerated_adverb(w));
    Some((words.len(), all_listed))
}

/// Whether the fronted adverb stack standing in
/// `tokens[..j]`, if one stands there, is
/// tolerated (round 20, R20-5): the stack is the
/// run of adverbial words in the adverb slot —
/// directly before the subject, or in the
/// pre-auxiliary slot the walk stopped at — and
/// it tolerates exactly when every word of it is
/// on the tolerated list and it plus the
/// `adverbs_used` already crossed by the walk
/// fits the one two-word cap. An unlisted
/// adverbial word in the slot, or a third listed
/// one, is not tolerated.
pub(super) fn fronted_stack_ok(tokens: &[String], j: usize, adverbs_used: usize) -> bool {
    // The auxiliary chain may wear an auxiliary the
    // walk did not cross — a modal standing before
    // a crossed auxiliary ("it will be postponed"):
    // those are the construction's own words, not a
    // fronted stack.
    let mut end = j;
    while end > 0 && AUXILIARY_WORDS.contains(&tokens[end - 1].as_str()) {
        end -= 1;
    }
    if end == 0 {
        return true;
    }
    // Where the stack stands, when one does: if the
    // run's last word is adverbial, the walk stopped
    // at the stack itself — the pre-auxiliary slot
    // ("it yesterday was moved": the subject stands
    // before the run). Otherwise the run's last word
    // is the subject (wearing any determiners or
    // possessives directly before it), and the stack
    // is the adverbial run directly before the
    // subject — the detached and in-clause fronted
    // positions. Words before that run are earlier
    // material (a governor clause, another clause's
    // words), not this construction's stack.
    let stack_end = if adverbial_word(&tokens[end - 1]) {
        end
    } else {
        let mut subj = end - 1;
        while subj > 0
            && (matches!(tokens[subj - 1].as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&tokens[subj - 1].as_str()))
        {
            subj -= 1;
        }
        subj
    };
    let mut stack_start = stack_end;
    while stack_start > 0 && adverbial_word(&tokens[stack_start - 1]) {
        stack_start -= 1;
    }
    let mut total = adverbs_used;
    for w in &tokens[stack_start..stack_end] {
        if is_tolerated_adverb(w) {
            total += 1;
            if total > 2 {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

/// The backward auxiliary / adverb walk the three
/// adverb scans share (round 20, R20-5): starting
/// just before `start`, cross auxiliaries and
/// tolerated adverbs in either order — the
/// post-auxiliary stack ("it was just moved") and
/// the pre-auxiliary stack ("it just was moved")
/// are one walk — under ONE budget of two
/// tolerated adverbs for the whole walk, and then
/// judge the fronted stack standing before the
/// subject by the same list and the same cap
/// (`fronted_stack_ok`). Returns the index the
/// walk stopped at and the adverbs it crossed, or
/// None when no auxiliary stands in the walk, a
/// third tolerated adverb stands in it, or the
/// fronted stack is not tolerated. Before round
/// 20 each scan walked on its own, the fronted
/// position was never judged at all, and a stack
/// split across the auxiliary spent past the cap.
pub(super) fn aux_adverb_walk(tokens: &[String], start: usize) -> Option<(usize, usize)> {
    aux_adverb_walk_bounded(tokens, start, 0, true)
}

/// `aux_adverb_walk` with the fronted stack judged
/// only within `tokens[bound..]`: a caller whose
/// run carries earlier material that is not this
/// construction's clause (an earlier date's span,
/// an earlier clause) bounds the fronted judgment
/// to the construction's own clause, exactly as
/// the unbounded walk's callers' runs are already
/// clause-scoped.
/// Whether an auxiliary stands just beyond the
/// slot words ending at `pos` (round 23, R23-4):
/// scanning back from `pos` crosses only slot
/// words (tolerated adverbs, adverbs of any kind,
/// "ever" / "yet", negations) and reaches an
/// auxiliary. The spending walk crosses a
/// negation only on this condition: a negation
/// in an AUXILIARY's slot is spent on its verb,
/// but a fronted negation with no auxiliary
/// beyond it ("not moved to November 8") is a
/// governor standing before its phrase, and the
/// walk must not pull it into the claim zone.
pub(super) fn aux_beyond_slot(tokens: &[String], pos: usize) -> bool {
    let mut k = pos;
    while k > 0 {
        let w = tokens[k - 1].as_str();
        if w.is_empty() {
            // Round 27 (R27-A, F3): nothing stands
            // in the slot — crossed as nothing.
            k -= 1;
            continue;
        }
        if AUXILIARY_WORDS.contains(&w) {
            return true;
        }
        if is_tolerated_adverb(w)
            || adverbial_word(w)
            || matches!(w, "not" | "never" | "ever" | "yet")
        {
            k -= 1;
        } else {
            return false;
        }
    }
    false
}

pub(super) fn aux_adverb_walk_bounded(
    tokens: &[String],
    start: usize,
    bound: usize,
    require_aux: bool,
) -> Option<(usize, usize)> {
    let mut j = start;
    let mut aux_seen = false;
    let mut adverbs_used = 0usize;
    // Round 23 (R23-4): a negation in the slot is
    // spent on its verb in every scan, this walk
    // included — "(it was not moved for November
    // 1)" asserts its occasion exactly as "(it was
    // moved for November 1)" does. The negation
    // counts toward the walk's two-word cap like
    // any slot word, so a three-word slot (a
    // negation and two adverbs, in any order) does
    // not spend, as a three-adverb slot never has.
    // An UNLISTED adverb crosses only beside a
    // negation — the negation already crossed
    // nearer the verb, or the negation standing
    // just past it — the passive constructions'
    // analogue of R22-4's single-unlisted slot:
    // "not loudly", "loudly never", and "not ever"
    // spend; an unlisted adverb alone ("loudly
    // moved") and two unlisted adverbs still do
    // not, the tolerated list governing which
    // adverbs spend as it always has.
    let mut negation_seen = false;
    while j > 0 {
        let prev = tokens[j - 1].as_str();
        if prev.is_empty() {
            // Round 27 (R27-A, F3): a token that
            // cleans to nothing in the auxiliary's
            // slot ("it was \u{200b} moved to
            // November 8") is nothing — crossed
            // without counting toward the cap.
            j -= 1;
            continue;
        }
        if AUXILIARY_WORDS.contains(&prev) {
            aux_seen = true;
            j -= 1;
        } else if matches!(prev, "not" | "never") && aux_beyond_slot(tokens, j - 1) {
            if adverbs_used >= 2 {
                return None;
            }
            adverbs_used += 1;
            negation_seen = true;
            j -= 1;
        } else if is_tolerated_adverb(prev)
            || ((adverbial_word(prev) || matches!(prev, "ever" | "yet"))
                && (negation_seen || (j >= 2 && matches!(tokens[j - 2].as_str(), "not" | "never"))))
        {
            if adverbs_used >= 2 {
                return None;
            }
            adverbs_used += 1;
            j -= 1;
        } else {
            break;
        }
    }
    if require_aux && !aux_seen {
        return None;
    }
    if j > bound && !fronted_stack_ok(&tokens[bound..], j - bound, adverbs_used) {
        return None;
    }
    Some((j, adverbs_used))
}

/// Whether the governor phrase standing at `i` in
/// `clause` is a SPENT predicate (round 16, R16-3):
/// a cancellation phrase — displacement and
/// cancellation alike, single and phrasal — standing
/// as the predicate of an auxiliary construction:
/// scanning back from the phrase crosses at most two
/// tolerated adverbs (round 18, R18-10) and then at
/// least one auxiliary
/// word ("it was moved", "they are moved", "it is
/// being moved", "it has moved", "it got moved", "it
/// was just moved", "it was put back"). Such a phrase
/// is spent on its subject whatever the verb, the
/// held "(it was moved, November 1)" garnish reading
/// generalized: the following date is the group's
/// occasion date. Round 15's rule was this shape
/// scoped to single displacement stems after five
/// listed auxiliaries; the boundary was arbitrary,
/// as the round-16 review states.
pub(super) fn is_spent_predicate(clause: &[String], i: usize, phrase: &[String]) -> bool {
    is_spent_predicate_fronted(clause, i, phrase, false)
}

/// `is_spent_predicate` with the fronted-stack
/// judgment of round 20 (R20-5): the backward walk
/// is the one shared walk (`aux_adverb_walk`) —
/// the same tolerated list and the same two-word
/// cap in the post-auxiliary, pre-auxiliary, and
/// fronted positions. `fronted_invalid` carries
/// the judgment for a fronted stack standing in
/// its OWN clause before this one, which the
/// clause slice itself cannot see: the caller
/// that sliced the clause (the governor window
/// walk) judges that stack and reports it here —
/// an invalid fronted stack leaves the predicate
/// unspent, exactly as an invalid in-clause stack
/// does.
pub(super) fn is_spent_predicate_fronted(
    clause: &[String],
    i: usize,
    phrase: &[String],
    fronted_invalid: bool,
) -> bool {
    // Round 28 (R28-A, F2): the phrase arrives as
    // words (see `word_phrase`), as at every phrase
    // matcher this round converts.
    if !is_cancellation_phrase(phrase) {
        return false;
    }
    if fronted_invalid {
        return false;
    }
    aux_adverb_walk(clause, i).is_some()
}

/// Whether a clause ending at a clause break holds
/// an UNSPENT governor as its trailing element
/// (round 13, R13-2): a governor phrase after which
/// nothing stands in the clause but date-referring
/// filler ("the", "date", "it"). Such a governor
/// has governed no date yet, so the break does not
/// erase it. A cancellation standing in the clause
/// directly after a date is the exception: it was
/// spent on that date by `governed_after`
/// ("(November 1, cancelled, November 8)" still
/// asserts November 8), so it carries no further.
/// `unspent_trailing_governor` with a fronted
/// stack standing in its own clause before this
/// one already judged invalid (round 20, R20-5;
/// see `is_spent_predicate_fronted`): a predicate
/// the fronted stack leaves unspent is an unspent
/// governor of this clause like any other.
pub(super) fn unspent_trailing_governor_fronted(
    clause: &[String],
    clause_follows_a_date: bool,
    fronted_invalid: bool,
) -> bool {
    for i in 0..clause.len() {
        if clause[i].is_empty() {
            continue;
        }
        // Round 31 (R31-C, F4): the phrase-start
        // gate, as in `governed_after`.
        if !governor_phrase_may_start(&clause[i]) {
            continue;
        }
        for len in 1..=3usize {
            // Round 28 (R28-A, F2): word-count
            // phrases, and a suffix of nothings is
            // no suffix — the words after the phrase
            // are its words that are something.
            let Some((phrase, phrase_end)) = word_phrase(clause, i, len) else {
                break;
            };
            if !phrase_governs(clause, i, &phrase) {
                continue;
            }
            // Round 15 (R15-1), generalized round 16
            // (R16-3): a governor participle standing
            // as an auxiliary's predicate ("it was
            // moved", "it was cancelled", "it was put
            // back") is that clause's own predicate,
            // already spent on its subject — the held
            // "(it was moved, November 1)" shape names
            // its date, and so do its synonyms.
            if is_spent_predicate_fronted(clause, i, &phrase, fronted_invalid) {
                continue;
            }
            // Round 14 (R14-2): the spent exception
            // covers the whole cancellation family,
            // not just cancelled/canceled — a trailing
            // "postponed" or "drop it" standing after
            // a date was spent on that date by
            // `governed_after`, the same as the
            // participles always were.
            if clause_follows_a_date && is_cancellation_phrase(&phrase) {
                continue;
            }
            let suffix = &clause[phrase_end..];
            if suffix
                .iter()
                .all(|w| w.is_empty() || matches!(w.as_str(), "the" | "date" | "it"))
            {
                return true;
            }
        }
    }
    false
}

/// Whether `word` can stand as the FIRST word of
/// a cancellation or governor phrase (round 31,
/// R31-C, F4): the first words of the families as
/// `is_cancellation_phrase` and `is_governor_phrase`
/// state them — a single governor whole (hyphenated
/// compounds included, by `is_single_governor`'s own
/// split test) or carrying a hyphenated cancellation
/// inside it, a cancellation verb (which also opens
/// the verb-and-object pairs), a phrasal-pair verb
/// in any of its forms (call / put / push / forget),
/// and the listed pairs' and triples' first words
/// ("no", "don't", "didn't", "dont", "didnt",
/// "rather", "instead", "never", "old", "hold",
/// "used", "do"). A word outside this set begins
/// no phrase of either family in any length, so a
/// scan that meets it can skip the phrase machinery
/// for that position without changing any judgment.
pub(super) fn governor_phrase_may_start(word: &str) -> bool {
    if is_single_governor(word) || is_cancellation_verb(word) {
        return true;
    }
    if word.contains('-') {
        let parts: Vec<String> = word
            .split('-')
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        if parts.len() >= 2 && hyphenated_cancellation(&parts) {
            return true;
        }
    }
    is_verb_form(word, "call")
        || is_verb_form(word, "put")
        || is_verb_form(word, "push")
        || is_verb_form(word, "forget")
        || matches!(
            word,
            "no" | "don't"
                | "didn't"
                | "dont"
                | "didnt"
                | "rather"
                | "instead"
                | "never"
                | "old"
                | "hold"
                | "used"
                | "do"
        )
}

/// Whether a governor standing AFTER a date inside
/// its group disqualifies that date (round 12,
/// R12-7a; parity completed round 14, R14-2): a
/// cancellation phrase in the words between this
/// date and the next one — or the group's end —
/// cancels the date it follows, and a forward
/// governor ("not", "no longer") standing after the
/// group's LAST date, with no later date left for
/// it to govern, falls back on the date it follows:
/// "(November 1, cancelled)", "(November 1, drop
/// it)", and "(November 1, not)" contribute
/// nothing. The held case stands apart by the same
/// rule: in "(November 1, not October 25)" the
/// trailing "not" has a date after it to govern, so
/// November 1 is untouched.
///
/// Round 15 (R15-6): the trailing reach is
/// clause-scoped — a phrase governs the date it
/// follows only when every clause between that
/// date and the phrase is itself a cancellation
/// or filler clause. A content clause breaks the
/// reach: "(November 1, choir practice, ignore)"
/// now contributes November 1, because the
/// "ignore" stands behind words that are neither.
pub(super) fn governed_after(
    raw_tokens: &[&str],
    tokens: &[String],
    date_end: usize,
    next_start: Option<usize>,
) -> bool {
    let seg_start = date_end.min(tokens.len());
    let seg_end = next_start.unwrap_or(tokens.len()).min(tokens.len());
    if seg_start >= seg_end {
        return false;
    }
    // The clause starts inside the segment: a new
    // clause opens after every clause break.
    // Round 28 (R28-A, F2 = review MINOR-3): the
    // governance break test (see
    // `governance_clause_break_after_at`) — a
    // nothing standing after a prospective in this
    // segment ("November 1, not * November 8")
    // opens no clause here either, so the
    // prospective is not spent backward as a
    // complete clause of its own; a break after a
    // spent cancellation counts as it always has.
    let mut clause_starts: Vec<usize> = vec![seg_start];
    for j in seg_start + 1..seg_end {
        if governance_clause_break_after_at(raw_tokens, tokens, j - 1) {
            clause_starts.push(j);
        }
    }
    let after = &tokens[seg_start..seg_end];
    for i in 0..after.len() {
        if after[i].is_empty() {
            continue;
        }
        // Round 31 (R31-C, F4): a word that can
        // begin no cancellation or governor phrase
        // governs nothing in any length, so the
        // phrase machinery is not run for it — the
        // scan used to build and judge three phrases
        // at every word of the segment, and a header
        // core carrying thousands of ordinary words
        // after its date (the conjoined-groups core
        // is mostly "and") cost time per word. The
        // gate is the families' own first words,
        // stated in `governor_phrase_may_start`;
        // every word it passes is judged exactly as
        // before.
        if !governor_phrase_may_start(&after[i]) {
            continue;
        }
        for len in 1..=3usize {
            // Round 28 (R28-A, F2): word-count
            // phrases, as at every phrase matcher
            // this round converts — the phrase's
            // length below is its word count.
            let Some((phrase, _phrase_end)) = word_phrase(after, i, len) else {
                break;
            };
            let phrase: &[String] = &phrase;
            // Round 14 (R14-2): a cancellation phrase
            // standing after a date cancels the date
            // it follows — the trailing position draws
            // from the same family as the before
            // position, not the cancelled/canceled
            // pair alone. Round 15 (R15-6): subject
            // to the clause scope stated above.
            // The rest of the family is prospective:
            // standing between two dates it governs
            // the later one (judged by that date's own
            // window), and only falls back on the date
            // it follows when no later date is left —
            // the standing "not" / "no longer" rule,
            // now read off the one family definition.
            // The clause the phrase stands in, judged
            // with the scope rule below.
            let pos = seg_start + i;
            let clause_idx = clause_starts.iter().rposition(|s| *s <= pos).unwrap_or(0);
            let own_start = clause_starts[clause_idx];
            let own_end = clause_starts
                .get(clause_idx + 1)
                .copied()
                .unwrap_or(seg_end);
            let own_clause = &tokens[own_start..own_end];
            // Round 16 (RT16-6): a determiner-headed
            // -s word is a noun phrase's head in the
            // trailing position too — "(November 1,
            // the drops)" names its date.
            // Round 16 (R16-2): a prospective phrase
            // standing as its OWN complete clause
            // between two dates — the clause ends at
            // a clause break inside the segment —
            // spends on the date it follows, together
            // with the run's cancellation clauses:
            // "(November 1, not, drop it, November 8)"
            // governs November 1 by the run. A
            // prospective word standing in the next
            // date's own clause ("(November 1, not
            // November 8)") keeps the standing rule:
            // it governs the later date, judged by
            // that date's window, and this date is
            // untouched.
            let own_clause_complete = own_end > own_start
                && governance_clause_break_after_at(raw_tokens, tokens, own_end - 1);
            let prospective_governs = next_start.is_some()
                && phrase_governs(after, i, phrase)
                && !is_cancellation_phrase(phrase)
                && own_clause_complete
                && is_prospective_only_clause(own_clause);
            let governs = (is_cancellation_phrase(phrase)
                && !(phrase.len() == 1 && determiner_headed_s_form(after, i, &phrase[0])))
                || (next_start.is_none() && phrase_governs(after, i, phrase))
                || prospective_governs;
            if !governs {
                continue;
            }
            // Round 19 (R19-1): a cancellation phrase
            // standing in a clause of event content
            // that runs directly into the NEXT date —
            // no clause break closes the clause before
            // that date stands — is the next date's own
            // clause governor, judged by that date's
            // window, not a governor of the date it
            // follows: in "(early service November 1,
            // late service cancelled November 1)" the
            // "cancelled" cancels the late service's
            // November 1, and the early service's date
            // is untouched by it. The standing
            // between-dates rule is untouched where it
            // has always applied: the phrase's clause
            // is closed by a break before the next
            // date, or the clause is a governor/filler
            // clause of its own ("(November 1,
            // cancelled November 8)" still governs
            // November 1 here as before).
            if next_start.is_some()
                && is_cancellation_phrase(phrase)
                && !own_clause_complete
                && !is_governor_family_clause(own_clause)
            {
                continue;
            }
            // Every clause between the date and the
            // phrase's clause, judged by the one scope
            // rule — round 16 (R16-2): the governor
            // families compose, so a run mixing
            // cancellation, prospective, and filler
            // clauses is in scope as a whole.
            let mut in_scope = true;
            for k in 0..clause_idx {
                let c_start = clause_starts[k];
                let c_end = clause_starts.get(k + 1).copied().unwrap_or(seg_end);
                let clause = &tokens[c_start..c_end];
                if !is_governor_family_clause(clause) {
                    in_scope = false;
                    break;
                }
            }
            if in_scope {
                return true;
            }
        }
    }
    false
}

/// Whether a weekday word standing directly before a
/// date inside its group contradicts that date
/// (round 11, R11-8): the same wrong-weekday-yields-
/// no-date rule the line readers obey, applied
/// inside groups — "(Monday November 1)" names an
/// impossible day, since November 1, 2026 is a
/// Sunday, and contributes nothing.
pub(super) fn contradicted_before(tokens: &[String], date_start: usize, date: SimpleDate) -> bool {
    // Round 27 (R27-A, F3): the weekday word
    // standing before the date is the nearest word
    // that is something before it — "(Monday
    // \u{200b} November 1)" contradicts exactly as
    // "(Monday November 1)" does.
    let prev = prev_nonempty(tokens, date_start, 1);
    let Some(&idx) = prev.first() else {
        return false;
    };
    match weekday_word(&tokens[idx]) {
        Some(named) => named != date.weekday(),
        None => false,
    }
}

/// The partial date an "or" tail offers inside a
/// group (round 11, R11-6): "(November 1 or the 8th)"
/// offers two dates, not one — the tail names only a
/// day, so it takes the month and year of the full
/// date standing before the "or".
pub(super) fn partial_or_alternative(
    tokens: &[String],
    qualified: &[(SimpleDate, usize, usize)],
) -> Option<SimpleDate> {
    for (i, w) in tokens.iter().enumerate() {
        if w != "or" {
            continue;
        }
        let base = qualified
            .iter()
            .rev()
            .find(|(_, start, _)| *start < i)
            .map(|(d, _, _)| *d)?;
        let mut j = i + 1;
        if tokens.get(j).map(String::as_str) == Some("the") {
            j += 1;
        }
        let day_token = tokens.get(j)?;
        let mut digits: &str = day_token.as_str();
        for suffix in ["st", "nd", "rd", "th"] {
            if let Some(stripped) = digits.strip_suffix(suffix) {
                digits = stripped;
                break;
            }
        }
        let digits: String = digits.to_string();
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let day: u32 = digits.parse().ok()?;
        let candidate = SimpleDate {
            year: base.year,
            month: base.month,
            day,
        };
        if candidate.is_valid() && candidate != base {
            return Some(candidate);
        }
    }
    None
}

/// A target-rule governor phrase (round 16, R16-1):
/// a displacement phrase, or a form of "postpone" —
/// the phrases whose whole meaning is a move to
/// another date.
pub(super) fn is_target_phrase(phrase: &[String]) -> bool {
    let words: Vec<&String> = phrase.iter().filter(|w| !w.is_empty()).collect();
    is_displacement_phrase(phrase) || (words.len() == 1 && is_verb_form(words[0], "postpone"))
}

/// The group-level target claim standing for the
/// date occurrence `idx` (round 16, R16-1): the
/// group contains a target phrase and a "to" /
/// "until" naming THIS date as the phrase's target —
/// the date is the first date standing after that
/// "to" / "until", the words between it and the date
/// are nothing but determiners, and the words between
/// the phrase and the "to" are an object noun phrase,
/// "back", or the like: no governor, cancellation
/// verb, auxiliary, or weekday stands among them.
/// Round 17 (R17-5): the reach from the phrase to
/// its "to" is bounded by the phrase's CLAUSE, not
/// by a word count — the round-16 four-word fence
/// lost a five-word object ("moved the November
/// potluck planning meeting to November 8")
/// silently. A clause break standing among the
/// in-between words ends the reach (a comma inside
/// or closing an intervening DATE is that date's
/// own punctuation, not a clause break in the
/// reach: the
/// second "to" of a correction — "moved to
/// November 8, then to November 15" — still claims
/// its date, round 17 R17-1). Round 17 (R17-3): a
/// "from" standing directly before a date among the
/// in-between words is part of the construction —
/// "(moved from November 1 to November 8)" claims
/// its target, and November 1 is governed and named
/// as the from-date like every other claimed
/// group's other date. Returns the claim zone: the
/// token the zone starts at — the phrase, extended
/// back over its auxiliaries and one tolerated
/// adverb, so the passive's "was" is part of the
/// construction, not a past marker governing the
/// target — and the "to" / "until" token.
pub(super) struct TargetClaim {
    pub(super) zone_start: usize,
    pub(super) to_pos: usize,
}

/// Whether `word` can stand as the FIRST word of
/// a target phrase (round 31, R31-C, F4): the
/// target family as `is_target_phrase` states it —
/// a displacement verb in any form (which is also
/// the one-word displacement phrase), a form of
/// "postpone", or a form of "put" / "push" opening
/// the "back" pairs. A word outside this set
/// begins no target phrase, so a scan that meets
/// it can skip the phrase build for that position
/// without changing any claim.
pub(super) fn target_phrase_may_start(word: &str) -> bool {
    is_displacement_verb(word)
        || is_verb_form(word, "postpone")
        || is_verb_form(word, "put")
        || is_verb_form(word, "push")
}

/// The shared tables one run of target-claim
/// judgments is read off (round 32, R32-C, F3).
/// The claim test for one occurrence used to
/// rescan the whole token run for every
/// occurrence it judged: walking back over every
/// token before the date hunting a "to" / "until",
/// rebuilding every target phrase before that
/// word, and answering each span / start / clause-
/// break question by scanning the occurrence list
/// or the run again. A run holding thousands of
/// occurrences — one big group holding thousands
/// of dates, a header line of thousands of
/// bracketed dates standing as plain signals —
/// paid that whole run once per occurrence, and
/// the parse cost quadratic time. Every table
/// here states, once, an answer the claim test
/// used to recompute per occurrence, computed by
/// the same predicates in the same order: the
/// "to" / "until" positions in written order; the
/// occurrence starts as a prefix count and a
/// per-index flag; the token indexes standing
/// inside an occurrence's span; the target
/// phrases standing in the run, kept per phrase
/// length (a phrase's end rises strictly with its
/// start over the run's words that are something,
/// so the phrases ending at or before a given
/// "to" form a prefix of each list); the
/// next-nonempty table; the last clause break
/// before each index; and the largest occurrence
/// end at or before each index. The claim
/// judgment below reads only these tables and
/// its occurrence's own neighbourhood; no answer
/// changes.
pub(super) struct TargetClaimScan {
    to_positions: Vec<usize>,
    starts_prefix: Vec<usize>,
    is_start: Vec<bool>,
    in_span: Vec<bool>,
    cand1: Vec<(usize, usize)>,
    cand2: Vec<(usize, usize)>,
    nonempty_next: Vec<Option<usize>>,
    last_break: Vec<Option<usize>>,
    largest_end: Vec<Option<usize>>,
}

pub(super) fn target_claim_scan(
    raw_tokens: &[&str],
    tokens: &[String],
    positional: &[(SimpleDate, usize, usize)],
) -> TargetClaimScan {
    let n = tokens.len();
    let to_positions: Vec<usize> = (0..n)
        .filter(|&i| matches!(tokens[i].as_str(), "to" | "until"))
        .collect();
    let mut is_start = vec![false; n];
    let mut in_span = vec![false; n];
    for (_, s, used) in positional {
        if *s < n {
            is_start[*s] = true;
        }
        for slot in in_span.iter_mut().take((*s + *used).min(n)).skip(*s) {
            *slot = true;
        }
    }
    let mut starts_prefix: Vec<usize> = vec![0; n + 1];
    for i in 0..n {
        starts_prefix[i + 1] = starts_prefix[i] + is_start[i] as usize;
    }
    let mut end_present = vec![false; n + 1];
    for (_, s, used) in positional {
        let end = s + used;
        if end <= n {
            end_present[end] = true;
        }
    }
    let mut largest_end: Vec<Option<usize>> = vec![None; n + 1];
    let mut best_end: Option<usize> = None;
    for x in 0..=n {
        if end_present[x] {
            best_end = Some(x);
        }
        largest_end[x] = best_end;
    }
    let mut last_break: Vec<Option<usize>> = vec![None; n + 1];
    let mut last: Option<usize> = None;
    for j in 0..n {
        if clause_break_after_at(raw_tokens, tokens, j) {
            last = Some(j);
        }
        last_break[j + 1] = last;
    }
    // The run's target phrases, per phrase length:
    // at each word that can open one (the R31-C
    // first-word gate, exactly as the per-occurrence
    // scan applied it), the phrase of that length
    // and whether it is a target phrase. A one-word
    // phrase that does not stand (no word at the
    // position) closes the position to both
    // lengths, as the shipped scan's `break` did.
    let mut cand1: Vec<(usize, usize)> = Vec::new();
    let mut cand2: Vec<(usize, usize)> = Vec::new();
    for ps in 0..n {
        if tokens[ps].is_empty() || !target_phrase_may_start(&tokens[ps]) {
            continue;
        }
        let Some((phrase, pe)) = word_phrase(tokens, ps, 1) else {
            continue;
        };
        if is_target_phrase(&phrase) {
            cand1.push((ps, pe));
        }
        if let Some((phrase, pe)) = word_phrase(tokens, ps, 2) {
            if is_target_phrase(&phrase) {
                cand2.push((ps, pe));
            }
        }
    }
    let nonempty_next = next_nonempty_table(tokens);
    TargetClaimScan {
        to_positions,
        starts_prefix,
        is_start,
        in_span,
        cand1,
        cand2,
        nonempty_next,
        last_break,
        largest_end,
    }
}

/// The target phrase the per-occurrence scan
/// finds for a "to" / "until" at `to_pos`, read
/// off the scan's phrase lists (round 32, R32-C):
/// the shipped scan walked every phrase start
/// before the "to" in written order and kept the
/// last phrase of either length that was a target
/// phrase ending at or before the "to" — the
/// phrase with the largest start, its two-word
/// form winning a tie at the same start. Each
/// list's ends rise strictly with its starts, so
/// each length's candidates ending at or before
/// `to_pos` form a prefix of its list, and the
/// answer is the later of the two prefixes' last
/// entries (the two-word entry on a tie).
fn target_phrase_found(scan: &TargetClaimScan, to_pos: usize) -> Option<(usize, usize)> {
    let c1 = {
        let upto = scan.cand1.partition_point(|(_, pe)| *pe <= to_pos);
        scan.cand1[..upto].last().copied()
    };
    let c2 = {
        let upto = scan.cand2.partition_point(|(_, pe)| *pe <= to_pos);
        scan.cand2[..upto].last().copied()
    };
    match (c1, c2) {
        (Some(a), Some(b)) => {
            if b.0 >= a.0 {
                Some(b)
            } else {
                Some(a)
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// The claim for every occurrence of a run,
/// judged against one shared scan (round 32,
/// R32-C): the batch entry the composition passes
/// use, where the shipped code mapped the
/// per-occurrence test over the whole run.
pub(super) fn target_claims(
    raw_tokens: &[&str],
    tokens: &[String],
    positional: &[(SimpleDate, usize, usize)],
) -> Vec<Option<TargetClaim>> {
    let scan = target_claim_scan(raw_tokens, tokens, positional);
    (0..positional.len())
        .map(|idx| target_claim_in(&scan, raw_tokens, tokens, positional, idx))
        .collect()
}

pub(super) fn target_claim_in(
    scan: &TargetClaimScan,
    raw_tokens: &[&str],
    tokens: &[String],
    positional: &[(SimpleDate, usize, usize)],
    idx: usize,
) -> Option<TargetClaim> {
    let start = positional[idx].1;
    for &to_pos in scan.to_positions.iter().rev() {
        if to_pos >= start {
            continue;
        }
        // Between the "to" and the date: determiners
        // only ("moved to the November 8").
        if !tokens[to_pos + 1..start]
            .iter()
            .all(|w| matches!(w.as_str(), "the" | "a" | "an" | ""))
        {
            continue;
        }
        // A date standing between this "to" and the
        // claimed date means the "to" names THAT date.
        // Round 32 (R32-C): read off the scan's start
        // prefix count — an occurrence starts strictly
        // between the "to" and the claimed date exactly
        // when the count of starts before the date
        // exceeds the count at or before the "to".
        if scan.starts_prefix[start] > scan.starts_prefix[to_pos + 1] {
            continue;
        }
        // The nearest target phrase ending at or
        // before the "to".
        // Round 28 (R28-A, F2): the phrase is read
        // in WORDS (see `word_phrase`) — a two-word
        // phrase wearing a nothing between its words
        // ("put \u{200b} back to …") is the phrase,
        // and its end is the index just past its
        // last word.
        // Round 32 (R32-C): read off the scan's
        // phrase lists (`target_phrase_found`), where
        // the shipped code rebuilt every phrase
        // before the "to" for every occurrence.
        let Some((ps, pe)) = target_phrase_found(scan, to_pos) else {
            continue;
        };
        // Between the phrase and the "to": an object
        // noun phrase, "back", or the like — no word
        // with a governing or verbal life of its own,
        // and the reach stays inside the phrase's
        // clause (round 17, R17-5 above). A "from"
        // directly before an intervening date is the
        // construction's own word (round 17, R17-3).
        let between = &tokens[pe..to_pos];
        // A comma standing inside an intervening
        // date's own span — after its day, before
        // its year, or closing the date — is that
        // date's punctuation, not a clause break in
        // the reach.
        // Round 18 (R18-4): a comma standing directly
        // BEFORE an intervening date is the object's
        // list comma in front of that date — "(moved
        // the meeting, November 1, 2026, and the
        // dinner to November 8)" writes its object as
        // a list whose middle item is the from-date.
        // The intervening date joins the claim's
        // governed set as the from-date; it never
        // breaks the reach by standing there, and
        // neither does the list comma that introduces
        // it. Its comma-after-date twin has always
        // claimed this way.
        let clause_bounded = (pe..to_pos).all(|j| {
            // Round 32 (R32-C): the span coverage and
            // the start-at-j+1 questions are the
            // scan's tables, exactly the predicates
            // they state.
            let in_date_span = scan.in_span[j];
            let before_intervening_date = j + 1 > pe && scan.is_start[j + 1];
            // Round 20 (R20-4): a comma standing on a
            // noun-like word directly before a verb-form
            // word the one verb test reads as a postposed
            // modifier ("dinner rolls, baked to …") is
            // that modifier phrase's own punctuation, not
            // a clause break in the reach — the postposed
            // adjective continues the object continuation
            // it postposes. A comma before a verb form the
            // test reads as finite (a real clause's verb)
            // keeps its standing clause-break force.
            // Round 21 (R21-10): the exemption is fenced
            // to that pinned postposed shape — the verb
            // form the comma's continuation contains,
            // reached by crossing only the noun-like
            // words of its own run ("beans, choir baked":
            // the continuation's verb form is "baked",
            // judged in its segment exactly as the
            // directly-postposed form is), and exempt
            // only when the one verb test reads THAT
            // form as a postposed modifier in its
            // segment. Before this fence, a two-noun
            // segment behind the comma ("beans," opening
            // a segment whose second noun heads a real
            // clause) left the comma standing as a break
            // the pinned shape's own logic should have
            // crossed the same way — the claim died, the
            // group offered both dates as alternatives,
            // and the plain path filed the vacated date.
            let postposed_modifier_break = noun_like_word(&tokens[j]) && {
                let mut k = j + 1;
                while k < to_pos
                    && tokens
                        .get(k)
                        .is_some_and(|w| noun_like_word(w) && !is_finite_verb_form(w))
                {
                    k += 1;
                }
                match tokens.get(k) {
                    Some(next) if k < to_pos && is_finite_verb_form(next) => {
                        // Round 32 (R32-C): the largest
                        // occurrence end at or before k,
                        // off the scan's table — the
                        // shipped scan filtered the
                        // whole occurrence list for it.
                        let seg_start = scan.largest_end[k].unwrap_or(pe);
                        let seg = &tokens[seg_start..to_pos];
                        !verb_form_is_finite(seg, k - seg_start)
                    }
                    _ => false,
                }
            };
            !clause_break_after_at(raw_tokens, tokens, j)
                || in_date_span
                || before_intervening_date
                || postposed_modifier_break
                // Round 26 (R26-C, M1): a token that
                // cleans to nothing is nothing here
                // either — its raw form may still
                // wear punctuation, and punctuation
                // on nothing breaks nothing.
                || tokens[j].is_empty()
        });
        let between_ok = clause_bounded
            && between.iter().enumerate().all(|(off, w)| {
                let j = pe + off;
                // Round 27 (R27-A, F3): the date the
                // "from" stands before is the next
                // word that is something after it — a
                // token that cleans to nothing
                // between "from" and its date ("moved
                // from \u{200b} November 1 to
                // November 8") does not detach the
                // "from" from the date it governs.
                // Round 32 (R32-C): the next word that
                // is something is the scan's
                // next-nonempty table (the R31-C
                // table), and "a date starts there" is
                // the scan's start flag.
                let next_nonempty = scan.nonempty_next[j + 1];
                let from_before_date =
                    w.as_str() == "from" && next_nonempty.is_some_and(|ns| scan.is_start[ns]);
                // Round 18 (R18-3): a word standing
                // inside an intervening SIGNAL's own
                // span — an earlier target in the same
                // claim chain, framed or relative — is
                // that target's own word, not a clause
                // word of this reach. A framed target's
                // words already passed this test (no
                // month or day word is a weekday word);
                // a relative target's "Sunday" is one,
                // so a chain whose earlier target was
                // written relatively ("moved to next
                // Sunday, then to November 15") lost
                // every later claim and the superseded
                // relative target asserted. The span
                // exemption covers the weekday test
                // only: governor, cancellation, and
                // auxiliary words still end the reach
                // wherever they stand.
                let in_signal_span = scan.in_span[j];
                // Round 20 (R20-6): a weekday word
                // followed by a noun-like word is a
                // noun phrase's modifier ("printed
                // Sunday bulletins"), not a clause
                // word of the reach — the same
                // reading the group signal scan
                // takes, so the reach crosses the
                // noun phrase it modifies.
                // Round 23 (R23-6): the one
                // weekday-modifier reading, verb-form
                // adjective shape included ("Sunday
                // taken seats" in an object
                // continuation is the phrase's own
                // material, and the claim crosses it).
                let weekday_modifier = weekday_modifier_at(tokens, j);
                from_before_date
                    || (!direct_single_governor(w)
                        && !is_cancellation_verb(w)
                        && !AUXILIARY_WORDS.contains(&w.as_str())
                        && (in_signal_span || weekday_word(w).is_none() || weekday_modifier))
            });
        if !between_ok {
            continue;
        }
        // Round 18 (R18-8): the reach's continuation
        // AFTER an intervening date is judged on its
        // own. The reach continues only into the
        // target construction itself — the "to"
        // directly ("moved November 1, to November
        // 8"), its lead-ins ("then to", "and to"),
        // or an object continuation of the phrase's
        // own object ("…, and the dinner to
        // November 8", the R18-4 witness: a
        // conjunction or determiner carrying the
        // object list on). A new finite clause
        // standing there — a subject and its finite
        // verb, "choir sang" — is clause content,
        // not object continuation, and it ends the
        // reach: "(moved November 1, choir sang to
        // November 8)" claims nothing for November
        // 8. The distinguisher is the finite verb:
        // object continuations name things, a second
        // clause does something. Only the words
        // after the intervening date's own span are
        // judged — the object BEFORE the date is the
        // phrase's object however it is written, and
        // a verb-repeated target ("moved November 1,
        // moved to November 8") re-anchors the phrase
        // itself, so no intervening date stands after
        // the phrase that claims.
        let clause_content_after_date = positional.iter().any(|(_, s, used)| {
            *s >= pe && s + used <= to_pos && segment_has_finite_verb(&tokens[s + used..to_pos])
        });
        if clause_content_after_date {
            continue;
        }
        // The claim zone extends back from the phrase
        // over its auxiliaries and up to two tolerated
        // adverbs (round 18, R18-10): "it was moved
        // to …" claims its target whole.
        // Round 20 (R20-5): the extension is the one
        // shared walk (`aux_adverb_walk_bounded`) —
        // the same list and the same two-word cap in
        // the post-auxiliary, pre-auxiliary, and
        // fronted positions, the fronted judgment
        // bounded to the phrase's own clause (after
        // the last clause break and after any earlier
        // date's span). A fronted stack standing in
        // its own clause before that bound is judged
        // by the same rule, as in the other scans.
        let bound = {
            // Round 32 (R32-C): both terms off the
            // scan's tables — the largest occurrence
            // end at or before the phrase (the shipped
            // scan's first loop, defaulting to zero),
            // and the last clause break before the
            // phrase (its second loop).
            let mut b = scan.largest_end[ps].unwrap_or(0);
            if let Some(j) = scan.last_break[ps] {
                b = b.max(j + 1);
            }
            b
        };
        let shared_walk = aux_adverb_walk_bounded(tokens, ps, bound, false);
        // Round 25 (R25-B, M8 = review R25-6): the
        // claim scan's own passive complement uses
        // the shared SLOT walk (`passive_for_slot_walk`),
        // as detection and passive-for already do
        // since R24-6 — the shared walk above crosses
        // an unlisted adverb only beside a negation,
        // so "(moved November 1, it was softly moved
        // to November 8)" kept its zone at the
        // participle (and its fronted judgment then
        // misread the uncrossed slot as an untolerated
        // fronted stack), the auxiliary's "was" stood
        // in the target's window as a past marker
        // governing it, and the claim died — while
        // its passive-for and detection twins
        // asserted. So the zone walk is the slot
        // walk's answer whenever the slot walk
        // reaches an auxiliary at or after the claim's
        // own bound and strictly further back than
        // the shared walk got, or the shared walk
        // failed outright: the zone extends over the
        // whole passive construction. A slot the
        // slot walk itself refuses — a three-word
        // slot, an untolerated fronted stack — still
        // kills the claim, exactly as before: the
        // slot walk's own cap and fronted judgment
        // are the same two-word rule.
        let slot_walk =
            passive_for_slot_walk(tokens, ps).filter(|(slot_start, _)| *slot_start >= bound);
        let (walk_start, adverbs_used) = match (shared_walk, slot_walk) {
            (Some((shared_start, _)), Some((slot_start, slot_used)))
                if slot_start < shared_start =>
            {
                (slot_start, slot_used)
            }
            (Some(shared), _) => shared,
            (None, Some(slot)) => slot,
            (None, None) => continue,
        };
        if bound > 0 {
            // Round 32 (R32-C): the last clause break
            // before the bound's own fragment, off the
            // scan's table — the start of the fragment
            // the bound closes.
            let pcs = scan.last_break[bound - 1].map(|j| j + 1).unwrap_or(0);
            if let Some((count, all_listed)) = adverb_stack_clause(&tokens[pcs..bound]) {
                if !all_listed || count + adverbs_used > 2 {
                    continue;
                }
            }
            // Round 21 (R21-9): a fronted stack
            // standing in its own clause before an
            // intervening date ("just really truly,
            // November 1, moved to …") is judged by
            // the same rule — the clause directly
            // before the bound's own fragment is the
            // stack's clause, exactly as the
            // governor-window walk judges the clause
            // directly before its window's clause.
            // The fragment check above sees only the
            // date's own fragment and missed it.
            if pcs > 0 {
                // Round 32 (R32-C): the same table,
                // one fragment earlier.
                let ppcs = scan.last_break[pcs - 1].map(|j| j + 1).unwrap_or(0);
                if let Some((count, all_listed)) = adverb_stack_clause(&tokens[ppcs..pcs]) {
                    if !all_listed || count + adverbs_used > 2 {
                        continue;
                    }
                }
            }
        }
        let zone_start = walk_start.max(bound);
        return Some(TargetClaim { zone_start, to_pos });
    }
    None
}

/// Whether the words in a target-claimed date's
/// governor window disqualify it (round 16, R16-1):
/// `governed_before`, except that a governor phrase
/// standing inside the claim zone — the target
/// phrase itself, its auxiliaries, its object — is
/// part of the target construction and does not
/// govern the target. A governor standing before
/// the zone ("not moved to …", "never moved to …")
/// still governs.
pub(super) fn governed_before_target(
    window: &[String],
    window_start: usize,
    claim: &TargetClaim,
) -> bool {
    for i in 0..window.len() {
        if window[i].is_empty() {
            continue;
        }
        // Round 31 (R31-C, F4): the phrase-start
        // gate, as in `governed_after`.
        if !governor_phrase_may_start(&window[i]) {
            continue;
        }
        for len in 1..=3usize {
            // Round 28 (R28-A, F2): word-count
            // phrases, as in `governed_before`.
            let Some((phrase, phrase_end)) = word_phrase(window, i, len) else {
                break;
            };
            if !phrase_governs(window, i, &phrase) {
                continue;
            }
            let abs_start = window_start + i;
            if abs_start >= claim.zone_start && abs_start <= claim.to_pos {
                continue;
            }
            // Round 15 (R15-1): the standing adjacency
            // skip, kept for phrases the claim zone
            // does not cover.
            if next_nonempty(window, phrase_end).is_some_and(|n| window[n].as_str() == "to")
                && is_displacement_phrase(&phrase)
            {
                continue;
            }
            return true;
        }
    }
    false
}

/// Whether one word may stand in a bare restatement
/// of a date (round 19, R19-1): the date's framing
/// words (a weekday word, "on", "the"), the
/// date-referring filler the governor machinery
/// already knows ("date", "it", "that", "note",
/// "notes", "of"), the connectives and prepositions
/// that only join a date to its clause ("and", "or",
/// "but", "then", "for", "to", "from", "at", "by",
/// "in"), an auxiliary, or a tolerated adverb. Any
/// other word — a label, a subject, an object, an
/// event verb — is event content of the occurrence's
/// own. ("for" here is only a joining word for this
/// test; the for-composition's own fences in
/// R18-5/R18-9 are judged elsewhere and unchanged.)
///
/// Round 21 (R21-8): the inventory is completed by
/// its general rule — FUNCTION WORDS, words that
/// name no event, are bare; content labels are not.
/// The classes that join on that rule: conjunction
/// and discourse heads ("so", "well", "however"),
/// the adverb "only" standing as a region word
/// (an adverb outside the tolerated list is
/// content only when it modifies an event; "only"
/// before a date modifies nothing), the
/// preposition family ("until", "before", "after",
/// "through", "circa", "about", "around", "near"),
/// and the possessive pronouns ("mine", "ours",
/// "yours", "hers", "theirs" — the determiner
/// forms were already here). The label heads the
/// R20-3 adjudication named (reminder, update,
/// correction, attention, see, btw, fyi, and
/// "thanks" in head position) stay content: they
/// head an announcement the office wrote, and a
/// restatement wearing one asserts. "too" and
/// trailing "thanks" are bare in the trailing
/// slot only (see `occurrence_is_bare`).
///
/// Round 22 (R22-8): the rule is class-based —
/// function words are bare, content labels are
/// not. The classes that join: the remaining
/// prepositions ("with", "during", "since",
/// "within", "without", "upon", "unto", "into",
/// "over", "under", "between", "among", "against",
/// "per", "via", "onto", "toward", "towards",
/// "across", "behind", "beyond", "beside",
/// "despite", "amid", and "as" — so the two-word
/// units "as per" and "as of" read bare word by
/// word, "of" being already here); the
/// determiner / quantifier class ("each", "every",
/// "either", "neither", "both", "all", "any",
/// "some", "another", "other", "same", "such");
/// possessive + "own" ("his own November 1" — the
/// possessives were already here, "own" joins);
/// the conjunction / discourse tail ("lest",
/// "albeit", "unless", "whereas", "hence", "thus",
/// "moreover", "furthermore", "nonetheless",
/// "nevertheless", "instead", "meanwhile",
/// "otherwise", "therefore", "anyway", "rather",
/// "quite", "indeed"); and "too", which is bare
/// in BOTH slots now (head position included —
/// "(too November 1 too)" reads nothing).
/// Trailing "thanks" stays bare in the trailing
/// slot only, and "thanks" in head position stays
/// content, per the R20-3 adjudication.
///
/// Round 23 (R23-1/R23-2): the CLASSES are the
/// rule; the lists below are their current
/// membership, not a closed set. A newly found
/// function word is a list gap — graded MAJOR
/// when the word is ordinary office vocabulary,
/// as every word in this round's classes is. The
/// classes that join: the deictics ("yesterday",
/// "today", "tomorrow", "here", "there",
/// "together", "alone", and the preposition "re"
/// — temporal/locative deictic adverbs and their
/// kin, the same class as the tolerated "again" /
/// "now" / "then", naming no event); the lanes'
/// enumerated remainders of the classes R22-8
/// named — prepositions ("past", "off", "besides",
/// "plus", "minus", "concerning", "regarding",
/// "barring", "following", "including", "atop",
/// "amidst", "amongst", "thru", "versus", "vs"),
/// quantifier / determiners ("much", "many",
/// "few", "several", "enough", "most", "more",
/// "less", "least", "none", "little", "what",
/// "which", "whose", "whatever", "whichever",
/// "whoever"), connective adverbs ("accordingly",
/// "consequently", "subsequently", "next",
/// "later", "earlier", "afterwards", "afterward",
/// "beforehand", "simultaneously", "eventually",
/// "initially"), and adverbs ("once", "twice",
/// "always", "ever", "perhaps", "maybe",
/// "almost", "nearly", "pretty", "fairly",
/// "basically", "merely"). The lanes' ungraded
/// pronouns / modals / interjections do NOT join.
///
/// Round 24 (R24-1): the subordinator class joins
/// the inventory — "when", "whenever", "while",
/// "whilst", "although", "though", "because",
/// "if", "yet", "provided", "supposing",
/// "wherever", "where", "why", "how", "whether"
/// — the commonest members of the conjunction
/// class the inventory already held in its rare
/// members ("lest", "albeit", "whereas",
/// "unless"), in head and trailing slots alike;
/// and "than" joins the connective class in the
/// same batch (review M1: "(more than November 1)"
/// restates bare through it). The mirror held for
/// every word: a content label standing with one
/// ("(our choir November 1)") still asserts. The
/// ungraded pronoun / modal / interjection sets
/// the red team recorded-not-graded stay
/// untouched.
///
/// Round 25 (R25-A): the classes complete further
/// on the same rule. F1 (RT25-1): the preposition
/// remainder — "till", "along", "excepting",
/// "save", "pending", "excluding" — and the
/// participial prepositions / subordinators
/// "considering", "assuming", "granted", "given"
/// join, and "anyhow" joins the connective adverbs
/// with its twin "anyway". F3 (RT25-3): the
/// pronoun class and the modal class join, graded
/// MAJOR in round 25 — the R24-A deferral pins
/// were deferrals, not adjudications, and flip.
/// M1 (RT25-5): the interjection / address class
/// joins the discourse garnish. M5 (review R25-9):
/// a small closed abbreviation class joins by
/// cleaned form. The mirror held for every
/// addition (a pronoun or modal with its verb —
/// "we sang" — still carries the verb as content,
/// and a content label standing with any of these
/// words still asserts).
///
/// Round 26 (R26-B): inventory ring II, on the same
/// rule. F3: the spatial-preposition remainder
/// ("above", "below", "inside", "outside",
/// "beneath", "underneath", "throughout",
/// "opposite", "alongside", "round",
/// "notwithstanding"). F5: the general adverb class
/// (47 words — frequency / office, formal
/// connectives, locative / directional, ordinal;
/// the bare inventory only, the aux-slot tolerated
/// list unwidened). F6: the do-paradigm ("do",
/// "does", "did", "done", "doing") joins the
/// auxiliary class as carried for this test. F7
/// (in part): the six recorded singletons ("ago",
/// "out", "up", "down", "away", "home") — pairs and
/// triples then partition word by word; the verb +
/// home phrases stay content by adjudication. M4:
/// the reflexive remainder ("myself", "yourself",
/// "ourselves", "yourselves", "oneself",
/// "whatsoever", "whosoever") and "nor". M5: the
/// quantifier / determiner remainder ("plenty",
/// "various", "numerous", "countless", "fewer",
/// "lots", "lot") and the number / ordinal words
/// ("first", "second", "third", "two", "three",
/// "four"), with the time-unit words the "… ago"
/// phrases carry ("day", "days", "week", "weeks",
/// "month", "months", "year", "years", "hour",
/// "hours", "time"). M6: the determiner-adjectives
/// ("whole", "entire", "certain", "particular",
/// "specific", "additional", "extra", "further",
/// "single", "sole", "half", "double"). The mirror
/// held for every class.
///
/// Round 27 (R27-B): inventory ring III, on the
/// same rule. B1: the time companions ("noon",
/// "midnight", "tonight", 29 kin, and the rest
/// sweep's strong singles). B2: the sign-off
/// singles of the graded core. B3: the lone title
/// words (title + name pairs do NOT join). B4:
/// the direction set only (the street/place
/// remainder is recorded, not fixed). B5: the
/// number paradigm remainder, both paradigms to
/// their end. B6: the pronoun tail's bare
/// singles. The mirror held for every class
/// (mirror_both_fail = 0 in every resurrecting
/// class of the lane's sweeps).
pub(super) fn bare_filler_word(w: &str) -> bool {
    weekday_word(w).is_some()
        || AUXILIARY_WORDS.contains(&w)
        || POSSESSIVE_DETERMINERS.contains(&w)
        || is_tolerated_adverb(w)
        || matches!(
            w,
            "the"
                | "a"
                | "an"
                | "on"
                | "for"
                | "of"
                | "date"
                | "it"
                | "that"
                // Round 20 (R20-3): the demonstratives
                // join "that", which was already here —
                // a restatement wearing only a
                // demonstrative ("these November 1")
                // carries no event content, exactly as
                // "the November 1" does not.
                | "this"
                | "these"
                | "those"
                | "note"
                | "notes"
                | "and"
                | "or"
                | "but"
                | "then"
                | "to"
                | "from"
                | "at"
                | "by"
                | "in"
                // Round 20 (R20-3): the discourse
                // garnish list is CLOSED — please, yes,
                // again, ps, and no others. These words
                // frame a restatement ("please
                // November 1", "ps November 1") without
                // naming any event; a word not on this
                // list (a label like "reminder", any
                // other discourse word) is still event
                // content for this test.
                | "please"
                | "yes"
                | "again"
                | "ps"
                // Round 21 (R21-8): the function-word
                // classes the general rule above
                // names — conjunction / discourse
                // heads, the adverb "only", the
                // preposition family, and the
                // possessive pronouns.
                | "so"
                | "well"
                | "however"
                | "only"
                | "until"
                | "before"
                | "after"
                | "through"
                | "circa"
                | "about"
                | "around"
                | "near"
                | "mine"
                | "ours"
                | "yours"
                | "hers"
                | "theirs"
                // Round 22 (R22-8): the remaining
                // prepositions, the determiner /
                // quantifier class, possessive
                // "own", the conjunction / discourse
                // tail, and "too" in both slots —
                // the classes named in the doc
                // comment above.
                | "with"
                | "during"
                | "since"
                | "within"
                | "without"
                | "upon"
                | "unto"
                | "into"
                | "over"
                | "under"
                | "between"
                | "among"
                | "against"
                | "per"
                | "via"
                | "onto"
                | "toward"
                | "towards"
                | "across"
                | "behind"
                | "beyond"
                | "beside"
                | "despite"
                | "amid"
                | "as"
                | "each"
                | "every"
                | "either"
                | "neither"
                | "both"
                | "all"
                | "any"
                | "some"
                | "another"
                | "other"
                | "same"
                | "such"
                | "own"
                | "lest"
                | "albeit"
                | "unless"
                | "whereas"
                | "hence"
                | "thus"
                | "moreover"
                | "furthermore"
                | "nonetheless"
                | "nevertheless"
                | "instead"
                | "meanwhile"
                | "otherwise"
                | "therefore"
                | "anyway"
                | "rather"
                | "quite"
                | "indeed"
                | "too"
                // Round 23 (R23-1): the deictic class —
                // temporal and locative deictics and
                // the preposition "re" (see the doc
                // comment above).
                | "yesterday"
                | "today"
                | "tomorrow"
                | "here"
                | "there"
                | "together"
                | "alone"
                | "re"
                // Round 23 (R23-2): the preposition
                // remainder.
                | "past"
                | "off"
                | "besides"
                | "plus"
                | "minus"
                | "concerning"
                | "regarding"
                | "barring"
                | "following"
                | "including"
                | "atop"
                | "amidst"
                | "amongst"
                | "thru"
                | "versus"
                | "vs"
                // Round 23 (R23-2): the quantifier /
                // determiner remainder.
                | "much"
                | "many"
                | "few"
                | "several"
                | "enough"
                | "most"
                | "more"
                | "less"
                | "least"
                | "none"
                | "little"
                | "what"
                | "which"
                | "whose"
                | "whatever"
                | "whichever"
                | "whoever"
                // Round 23 (R23-2): the connective-
                // adverb remainder.
                | "accordingly"
                | "consequently"
                | "subsequently"
                | "next"
                | "later"
                | "earlier"
                | "afterwards"
                | "afterward"
                | "beforehand"
                | "simultaneously"
                | "eventually"
                | "initially"
                // Round 23 (R23-2): the adverb
                // remainder.
                | "once"
                | "twice"
                | "always"
                | "ever"
                | "perhaps"
                | "maybe"
                | "almost"
                | "nearly"
                | "pretty"
                | "fairly"
                | "basically"
                | "merely"
                // Round 24 (R24-1): the subordinator
                // class (see the doc comment above),
                // and "than" with the connectives.
                | "when"
                | "whenever"
                | "while"
                | "whilst"
                | "although"
                | "though"
                | "because"
                | "if"
                | "yet"
                | "provided"
                | "supposing"
                | "wherever"
                | "where"
                | "why"
                | "how"
                | "whether"
                | "than"
                // Round 25 (R25-A, F1): the
                // preposition remainder and the
                // participial prepositions /
                // subordinators, and "anyhow" with
                // the connective adverbs.
                | "till"
                | "along"
                | "excepting"
                | "save"
                | "pending"
                | "excluding"
                | "considering"
                | "assuming"
                | "granted"
                | "given"
                | "anyhow"
                // Round 25 (R25-A, F3): the pronoun
                // class — the round-24 recorded set
                // and the additions the round-25
                // sweep found resurrecting ("i",
                // "who", "whom", "whomever", "one",
                // "others").
                | "i"
                | "we"
                | "they"
                | "you"
                | "he"
                | "she"
                | "him"
                | "them"
                | "us"
                | "me"
                | "itself"
                | "himself"
                | "herself"
                | "themselves"
                | "someone"
                | "anyone"
                | "everyone"
                | "somebody"
                | "anybody"
                | "everybody"
                | "something"
                | "anything"
                | "everything"
                | "nothing"
                | "nobody"
                | "who"
                | "whom"
                | "whomever"
                | "one"
                | "others"
                // Round 25 (R25-A, F3): the modal
                // class, as probed.
                | "will"
                | "would"
                | "can"
                | "could"
                | "may"
                | "might"
                | "must"
                | "shall"
                | "should"
                | "ought"
                | "dare"
                | "need"
                // Round 25 (R25-A, M1): the
                // interjection / address class,
                // discourse garnish of the same kind
                // as "please" / "yes" / "ps" above.
                | "oh"
                | "ah"
                | "oops"
                | "wow"
                | "hey"
                | "hi"
                | "hello"
                | "dear"
                | "yeah"
                | "ok"
                | "okay"
                | "amen"
                | "friends"
                | "folks"
                | "bye"
                | "welcome"
                | "sorry"
                | "greetings"
                | "congratulations"
                // Round 25 (R25-A, M5): the closed
                // abbreviation class, by cleaned
                // form — "St." cleans to "st", and
                // the common title set of a church
                // office.
                | "st"
                | "mt"
                | "mr"
                | "mrs"
                | "ms"
                | "dr"
                | "rev"
                | "fr"
                | "br"
                | "sr"
                | "jr"
                // Round 26 (R26-B, F3 = RT26-2): the
                // spatial-preposition remainder — the
                // core spatial prepositions whose
                // neighbours were all already in.
                | "above"
                | "below"
                | "inside"
                | "outside"
                | "beneath"
                | "underneath"
                | "throughout"
                | "opposite"
                | "alongside"
                | "round"
                | "notwithstanding"
                // Round 26 (R26-B, F5 = RT26-4): the
                // general adverb class joins the bare
                // inventory — membership exactly the
                // lane's 47-word sweep. Bare inventory
                // only: the aux-slot tolerated-adverb
                // list is a different surface and is
                // NOT widened.
                | "usually"
                | "generally"
                | "typically"
                | "frequently"
                | "sometimes"
                | "sometime"
                | "seldom"
                | "rarely"
                | "daily"
                | "weekly"
                | "monthly"
                | "annually"
                | "lately"
                | "presently"
                | "momentarily"
                | "instantly"
                | "directly"
                | "altogether"
                | "somehow"
                | "henceforth"
                | "forthwith"
                | "heretofore"
                | "thereby"
                | "hereby"
                | "wherein"
                | "thereafter"
                | "somewhere"
                | "anywhere"
                | "everywhere"
                | "nowhere"
                | "indoors"
                | "outdoors"
                | "upstairs"
                | "downstairs"
                | "aboard"
                | "abroad"
                | "ahead"
                | "aside"
                | "apart"
                | "overnight"
                | "underway"
                | "backwards"
                | "forwards"
                | "firstly"
                | "secondly"
                | "thirdly"
                | "lastly"
                // Round 26 (R26-B, F6 = RT26-5): the
                // do-paradigm joins the auxiliary class
                // as carried for the bare test (the
                // be / have / get paradigms reach this
                // test through `AUXILIARY_WORDS` above;
                // the claim machinery's own auxiliary
                // walks are a different surface and are
                // not widened).
                | "do"
                | "does"
                | "did"
                | "done"
                | "doing"
                // Round 26 (R26-B, F7 in part = RT26-6):
                // the six recorded singleton words join
                // — pairs and triples then partition
                // word by word. The verb + home phrase
                // family is recorded-NOT-fixed by
                // adjudication and does NOT join.
                | "ago"
                | "out"
                | "up"
                | "down"
                | "away"
                | "home"
                // Round 26 (R26-B, M4 = RT26-8): the
                // reflexive remainder and "nor" join
                // (himself / herself / itself /
                // themselves already shipped).
                | "myself"
                | "yourself"
                | "ourselves"
                | "yourselves"
                | "oneself"
                | "whatsoever"
                | "whosoever"
                | "nor"
                // Round 26 (R26-B, M5 = RT26-9): the
                // quantifier / determiner remainder and
                // the number / ordinal words join.
                | "plenty"
                | "various"
                | "numerous"
                | "countless"
                | "fewer"
                | "lots"
                | "lot"
                | "first"
                | "second"
                | "third"
                | "two"
                | "three"
                | "four"
                // Round 26 (R26-B, M5 serving F7): the
                // time-unit words the "… ago" phrases
                // carry — day / days / week / weeks /
                // month / months / year / years /
                // hour / hours / time ("while" is
                // already in; "long time ago" and
                // "a bit ago" partition through the
                // phrase units added with F4 / F7).
                | "day"
                | "days"
                | "week"
                | "weeks"
                | "month"
                | "months"
                | "year"
                | "years"
                | "hour"
                | "hours"
                | "time"
                // Round 26 (R26-B, M6 = RT26-10): the
                // determiner-adjectives join.
                | "whole"
                | "entire"
                | "certain"
                | "particular"
                | "specific"
                | "additional"
                | "extra"
                | "further"
                | "single"
                | "sole"
                | "half"
                | "double"
                // Round 27 (R27-B, B1 = RT27-B1): the
                // time-companion class joins — noon /
                // midnight / tonight and their kin,
                // the seasons included; 32 words, 64
                // of 64 lane slots resurrected, mirrors
                // asserting (bare27). The rest-sweep
                // strong singles join with them
                // (midday, midweek, nightly, hourly,
                // yearly). The timeword tail (eve,
                // morn, o'clock, pm, fortnight,
                // century, …) does NOT join — recorded
                // under B1's family. The event-label
                // nouns swept in the same class
                // (holiday, christmas, breakfast,
                // lunch, dinner, supper, …) do NOT
                // join — they are event labels.
                | "noon"
                | "midnight"
                | "tonight"
                | "morning"
                | "afternoon"
                | "evening"
                | "night"
                | "weekend"
                | "dawn"
                | "dusk"
                | "sunrise"
                | "sunset"
                | "daytime"
                | "nighttime"
                | "weekday"
                | "minute"
                | "moment"
                | "lunchtime"
                | "bedtime"
                | "daybreak"
                | "nightfall"
                | "twilight"
                | "mealtime"
                | "teatime"
                | "suppertime"
                | "dinnertime"
                | "spring"
                | "summer"
                | "autumn"
                | "fall"
                | "winter"
                | "season"
                | "midday"
                | "midweek"
                | "nightly"
                | "hourly"
                | "yearly"
                // "coming" joins for the swept
                // phrase "coming sunday": "Sunday
                // November 1" is a weekday-framed
                // occurrence whose region carries
                // "coming" alone, so no phrase unit
                // can reach that shape — the word
                // itself joins, as the R26-B F7
                // singletons did for their pairs.
                // Its mirror holds (bare27 re-run:
                // "(coming sunday choir November 1)"
                // still asserts). Membership
                // source, recorded honestly: the
                // B1 pair sweep listed the PAIR
                // "coming sunday" as resurrecting
                // under a FIX disposition, not the
                // lone word; the lone word joins
                // by coordinator adjudication
                // recorded in the round-27 close,
                // amending R27-B's membership
                // sourcing for this one word,
                // because the phrase-unit mechanism
                // cannot reach the shape above
                // (probed with the unit listed:
                // the head slot still asserted on
                // every surface). The unit is
                // therefore replaced by the bare
                // word (BARE_PHRASE_UNITS 166).
                | "coming"
                // Round 27 (R27-B, B2 = RT27-B2): the
                // sign-off singles of the graded core
                // join — 17 words (bare27: the class
                // resurrected 75 of 98 slots; the
                // abstract tail — grace, mercy, joy,
                // hope, faith, love, peace, prayers,
                // blessings, praise, glory, blessed,
                // hallelujah, hosanna, alleluia,
                // maranatha, shalom, and the "good
                // morning / afternoon / evening"
                // greetings — does NOT join: names
                // and labels in this corpus). "thanks"
                // joins for the head slot it
                // resurrected in; its trailing slot
                // already governed by the R20-3 rule.
                // This flips the R26-B pin's lone-head
                // "thanks" reading — flagged in the
                // R27-B addendum.
                | "sincerely"
                | "regards"
                | "faithfully"
                | "cordially"
                | "warmly"
                | "kindly"
                | "respectfully"
                | "gratefully"
                | "cheerfully"
                | "prayerfully"
                | "farewell"
                | "goodbye"
                | "goodnight"
                | "wishes"
                | "best"
                | "thank"
                | "thanks"
                // Round 27 (R27-B, B3 = RT27-B3): the
                // lone title words join — 46 words, 92
                // of 106 lane slots resurrected,
                // mirrors asserting (bare27). Title +
                // name PAIRS do NOT join as units
                // (binding split disposition): a named
                // person is the corpus's canonical
                // content label, and "(pastor lee
                // November 1)" asserting is the
                // accepted mirror direction.
                | "pastor"
                | "reverend"
                | "father"
                | "brother"
                | "sister"
                | "deacon"
                | "elder"
                | "bishop"
                | "chaplain"
                | "rabbi"
                | "priest"
                | "minister"
                | "missionary"
                | "professor"
                | "doctor"
                | "captain"
                | "major"
                | "colonel"
                | "general"
                | "sir"
                | "madam"
                | "madame"
                | "lord"
                | "lady"
                | "king"
                | "queen"
                | "prince"
                | "princess"
                | "saint"
                | "pope"
                | "president"
                | "chairman"
                | "secretary"
                | "treasurer"
                | "director"
                | "manager"
                | "coach"
                | "teacher"
                | "nurse"
                | "officer"
                | "judge"
                | "mayor"
                | "senator"
                | "miss"
                | "hon"
                | "esq"
                // Round 27 (R27-B, B4 = RT27-B4): the
                // direction set joins — 11 words (the
                // RT26-4 locative family in noun
                // form). The street/place remainder
                // (street, road, avenue, church,
                // chapel, school, hall, …) does NOT
                // join — recorded, not fixed
                // (binding): "(church November 1)" is
                // an event-label fragment.
                | "north"
                | "south"
                | "east"
                | "west"
                | "northeast"
                | "northwest"
                | "southeast"
                | "southwest"
                | "uptown"
                | "downtown"
                | "midtown"
                // Round 27 (R27-B, B5 = RT27-B5): the
                // number paradigm remainder joins to
                // its end — ordinals fourth…
                // hundredth + last / previous /
                // preceding / thrice / triple /
                // quarter (27 words, 54 of 68 lane
                // slots) and cardinals five…million
                // + dozen / zero (28 words, 56 of 58
                // lane slots), mirrors asserting.
                | "fourth"
                | "fifth"
                | "sixth"
                | "seventh"
                | "eighth"
                | "ninth"
                | "tenth"
                | "eleventh"
                | "twelfth"
                | "thirteenth"
                | "fourteenth"
                | "fifteenth"
                | "sixteenth"
                | "seventeenth"
                | "eighteenth"
                | "nineteenth"
                | "twentieth"
                | "thirtieth"
                | "fortieth"
                | "fiftieth"
                | "hundredth"
                | "last"
                | "previous"
                | "preceding"
                | "thrice"
                | "triple"
                | "quarter"
                | "five"
                | "six"
                | "seven"
                | "eight"
                | "nine"
                | "ten"
                | "eleven"
                | "twelve"
                | "thirteen"
                | "fourteen"
                | "fifteen"
                | "sixteen"
                | "seventeen"
                | "eighteen"
                | "nineteen"
                | "twenty"
                | "thirty"
                | "forty"
                | "fifty"
                | "sixty"
                | "seventy"
                | "eighty"
                | "ninety"
                | "hundred"
                | "thousand"
                | "million"
                | "dozen"
                | "zero"
                // Round 27 (R27-B, B6 = RT27-B6): the
                // pronoun tail's bare singles join —
                // noone, anymore, anyways, elsewhere
                // (the tail's fifth member, "anyone
                // else", joins as a phrase unit
                // below). The shipped pronoun matrix
                // held behind this tail; it is closed
                // by inspection.
                | "noone"
                | "anymore"
                | "anyways"
                | "elsewhere"
        )
}

/// Whether one side of an occurrence's region
/// carries a passive predicate construction
/// (round 20, R20-2): a cancellation/displacement
/// participle standing as an auxiliary's predicate
/// anywhere in the run — the `is_spent_predicate`
/// test, applied to every phrase in the run. A
/// lone auxiliary ("was November 1") and a lone
/// participle with no auxiliary ("November 1,
/// cancelled") are each no construction and do not
/// fire here.
pub(super) fn region_has_passive_predicate(words: &[String]) -> bool {
    (0..words.len()).any(|i| {
        // Round 28 (R28-A, F2): word-count phrases,
        // as at every phrase matcher this round
        // converts.
        (1..=3usize).any(|len| {
            word_phrase(words, i, len)
                .is_some_and(|(phrase, _)| is_spent_predicate(words, i, &phrase))
        })
    })
}

/// Whether the AFTER side of an occurrence's region
/// carries a passive predicate of the occurrence's
/// OWN (round 21, R21-5): an occurrence's regions end
/// at the neighbouring occurrences, so a passive
/// "for" construction standing in the after-region
/// whose occasion is the NEXT occurrence — the run
/// is the next date's lead-in, "it was moved for"
/// and then the next date stands — credits that next
/// occurrence, not this one. Such a lead-in does not
/// make this occurrence non-bare: "November 1, 2026,
/// cancelled, November 1, 2026, it was moved for
/// November 8, 2026" — the restated November 1 stays
/// a bare restatement, bound by the cancellation,
/// and November 8 asserts by its own construction.
/// A predicate that completes inside the region (the
/// comma form, "it was moved, November 8") is the
/// occurrence's own, exactly as before.
pub(super) fn after_region_has_own_passive_predicate(words: &[String], has_next: bool) -> bool {
    for i in 0..words.len() {
        for len in 1..=3usize {
            // Round 28 (R28-A, F2): word-count
            // phrases, and the lead-in's "for" is
            // the suffix's last word that is
            // something.
            let Some((phrase, phrase_end)) = word_phrase(words, i, len) else {
                break;
            };
            if !is_spent_predicate(words, i, &phrase) {
                continue;
            }
            if has_next {
                // The construction runs on into the
                // next occurrence: everything after
                // the participle phrase is joining
                // words ending in the "for" that
                // names the next date as its occasion.
                let suffix = &words[phrase_end..];
                let lead_in = !suffix.is_empty()
                    && last_word(suffix) == Some("for")
                    && suffix
                        .iter()
                        .all(|w| w.is_empty() || w.as_str() == "for" || bare_filler_word(w));
                if lead_in {
                    continue;
                }
            }
            return true;
        }
    }
    false
}

/// Round 24 (R24-2): the PHRASE units of the bare
/// test. A region is bare when its words partition
/// into inventory words and these listed phrase
/// units, matched as units — the closed list as
/// shipped is exactly: "care of", "in care of",
/// "care of the", "back together", "back together
/// again", "gone tomorrow", "gone by tomorrow",
/// "per se", "if and when", "when and if",
/// "as and when", "yet again", "together at last".
/// Their members are not individually function
/// words ("care" is a noun, "gone" a verb form,
/// "last" and "se" name nothing bare), so no word
/// list reaches them; a unit's member standing
/// alone keeps its current reading — "(care
/// November 1)" and "(gone November 1)" are not
/// bare, and assert as they did.
///
/// Round 25 (R25-A, F2 = RT25-2 + review R25-4):
/// the resurrecting units of the red team's sweep
/// join — the office core of complex prepositions
/// ("in spite of", "as long as", "on behalf of",
/// "in lieu of", "with regard to", and kin), the
/// idiom tail ("at last", "back home", "sooner or
/// later", and kin), and the subordinator phrases
/// ("as far as", "given that", "seeing that",
/// "save that"). Membership is the sweep's
/// resurrecting set exactly (bare25 / bare25b,
/// mirror-checked); longest-first matching
/// partitions "back together at last" whole, as
/// one listed unit, where the two shipped units
/// sharing "together" left no disjoint partition.
/// Members standing alone keep their readings, as
/// R24-2 required.
pub(super) const BARE_PHRASE_UNITS: &[&[&str]] = &[
    &["care", "of"],
    &["in", "care", "of"],
    &["care", "of", "the"],
    &["back", "together"],
    &["back", "together", "again"],
    &["gone", "tomorrow"],
    &["gone", "by", "tomorrow"],
    &["per", "se"],
    &["if", "and", "when"],
    &["when", "and", "if"],
    &["as", "and", "when"],
    &["yet", "again"],
    &["together", "at", "last"],
    // Round 25 (R25-A, F2): the office core.
    &["in", "spite", "of"],
    &["in", "spite", "of", "the"],
    &["as", "long", "as"],
    &["so", "long", "as"],
    &["in", "case"],
    &["just", "in", "case"],
    &["on", "behalf", "of"],
    &["on", "top", "of"],
    &["in", "front", "of"],
    &["in", "back", "of"],
    &["by", "way", "of"],
    &["for", "the", "sake", "of"],
    &["in", "light", "of"],
    &["in", "view", "of"],
    &["in", "lieu", "of"],
    &["with", "regard", "to"],
    &["in", "regard", "to"],
    &["ahead", "of"],
    &["ahead", "of", "time"],
    &["out", "of"],
    &["up", "to"],
    &["in", "time"],
    &["on", "time"],
    // Round 25 (R25-A, F2): the idiom tail.
    &["at", "last"],
    &["at", "long", "last"],
    &["together", "at", "long", "last"],
    &["back", "together", "at", "last"],
    &["back", "home"],
    &["back", "home", "together"],
    &["gone", "home"],
    &["gone", "home", "tomorrow"],
    &["gone", "for", "tomorrow"],
    &["back", "by", "tomorrow"],
    &["come", "back", "together"],
    &["at", "home"],
    &["long", "ago"],
    &["long", "last"],
    &["far", "away"],
    &["sooner", "or", "later"],
    &["back", "and", "forth"],
    &["to", "and", "fro"],
    &["in", "and", "out"],
    &["up", "and", "down"],
    &["one", "by", "one"],
    &["day", "by", "day"],
    &["first", "and", "last"],
    // Round 25 (R25-A, F2): the subordinator
    // phrases.
    &["as", "far", "as"],
    &["given", "that"],
    &["seeing", "that"],
    &["save", "that"],
    // Round 26 (R26-B, F4 = RT26-3): the complex-
    // preposition phrase ring, second turn — the
    // resurrecting units of the lane's sweep join,
    // membership exactly as swept (bare26, mirror-
    // checked). "thanks to" joins for the head slot
    // it resurrected in; its trailing behaviour is
    // unchanged (trailing "thanks" was already bare
    // by the R20-3 rule, "to" already bare).
    // (Round 27, R27-B / B2: "thanks" joins the
    // bare inventory itself, superseding the
    // lone-head reading recorded here in round 26.)
    &["in", "addition", "to"],
    &["in", "addition"],
    &["in", "accordance", "with"],
    &["in", "connection", "with"],
    &["in", "relation", "to"],
    &["with", "respect", "to"],
    &["in", "respect", "to"],
    &["in", "response", "to"],
    &["in", "reference", "to"],
    &["by", "means", "of"],
    &["in", "exchange", "for"],
    &["in", "place", "of"],
    &["in", "favor", "of"],
    &["in", "favour", "of"],
    &["on", "account", "of"],
    &["by", "reason", "of"],
    &["as", "a", "result", "of"],
    &["in", "the", "event", "of"],
    &["due", "to"],
    &["owing", "to"],
    &["thanks", "to"],
    &["prior", "to"],
    &["subsequent", "to"],
    &["contrary", "to"],
    &["as", "opposed", "to"],
    &["close", "to"],
    // Round 26 (R26-B, F4): the discourse / idiom
    // tail of the same sweep.
    &["at", "best"],
    &["at", "worst"],
    &["at", "first"],
    &["at", "present"],
    &["for", "good"],
    &["so", "far"],
    &["up", "until"],
    &["up", "till"],
    &["down", "to"],
    &["day", "after", "day"],
    &["week", "by", "week"],
    &["month", "by", "month"],
    &["year", "by", "year"],
    &["one", "at", "a", "time"],
    &["two", "by", "two"],
    &["first", "of", "all"],
    &["last", "of", "all"],
    &["above", "all"],
    &["in", "the", "end"],
    &["at", "the", "end"],
    &["in", "the", "meantime"],
    &["for", "the", "time", "being"],
    &["at", "the", "time"],
    &["at", "times"],
    &["in", "good", "time"],
    &["in", "the", "long", "run"],
    // Round 26 (R26-B, F7 in part): "long time ago"
    // joins as a unit so the "… ago" family
    // partitions ("two days ago", "a week ago", …)
    // once the number / time words below join —
    // "long" standing alone keeps its R25-A reading.
    &["long", "time", "ago"],
    // Round 26 (R26-B, F7 in part): "bit ago" joins
    // as a unit for "a bit ago" — "bit" standing
    // alone keeps its reading.
    &["bit", "ago"],
    // Round 27 (R27-B, B1 = RT27-B1): the time
    // companions' phrase ring. Most of the swept
    // units partition word by word once the B1
    // bare words above join ("at noon", "tomorrow
    // morning", "last night", "in the morning",
    // "by midnight", "every morning", "last
    // sunday", "previous sunday", "sunday noon",
    // "for tonight", …); the units whose members
    // do not join bare are listed here — "after
    // lunch" ("lunch" is an event label and keeps
    // its reading standing alone). "coming
    // sunday" needs no unit: "coming" joins the
    // bare words above and "sunday" is a weekday
    // word, so the phrase partitions word by word
    // — including in the weekday-framed shape,
    // where the date's region carries "coming"
    // alone.
    &["after", "lunch"],
    // Round 27 (R27-B, B2 = RT27-B2): the sign-off
    // pairs of the graded core whose members do
    // not partition word by word. The rest of the
    // core's pairs partition once the singles join
    // ("best regards", "sincerely yours", "yours
    // faithfully", "thank you", "thanks again",
    // "many thanks", "with thanks"). The recorded
    // letter tail ("dear friend", "with love",
    // "in christ", "in prayer", "your pastor",
    // "your friend", …) does NOT join.
    &["kind", "regards"],
    &["warm", "regards"],
    &["god", "bless"],
    &["bless", "you"],
    &["see", "you"],
    &["see", "you", "sunday"],
    // Round 27 (R27-B, B6 = RT27-B6): the closed
    // connective / phrase tail — the pronoun
    // tail's phrase member, the partitives, the
    // locative / devotional phrases, the time and
    // feast phrases, and the "other …" pair.
    // Members standing alone keep their readings:
    // "christmas" and "easter" are event labels
    // and "(christmas November 1)" still asserts.
    // "last time" / "fourth time" / "by night" /
    // "for tonight" partition word by word once
    // B5 / B1 join and are not listed. The B4
    // pair sweep's phrases join exactly where
    // they partition under this disposition —
    // "at church", "at school", and the partitives
    // below — while "main street", "in town",
    // and the "… side" phrases stay recorded with
    // B4's place-noun remainder.
    &["anyone", "else"],
    &["sooner", "than"],
    &["other", "words"],
    &["other", "hand"],
    &["long", "time"],
    &["short", "time"],
    &["part", "of"],
    &["rest", "of"],
    &["end", "of"],
    &["top", "of"],
    &["back", "of"],
    &["front", "of"],
    &["side", "of"],
    &["middle", "of"],
    &["bottom", "of"],
    &["on", "top"],
    &["at", "top"],
    &["in", "front"],
    &["in", "back"],
    &["at", "church"],
    &["at", "school"],
    &["at", "work"],
    &["at", "rest"],
    &["at", "peace"],
    &["in", "peace"],
    &["in", "love"],
    &["in", "faith"],
    &["in", "hope"],
    &["in", "grace"],
    &["by", "faith"],
    &["by", "grace"],
    &["by", "heart"],
    &["by", "hand"],
    &["by", "mail"],
    &["by", "phone"],
    &["by", "email"],
    &["by", "name"],
    &["for", "christmas"],
    &["for", "easter"],
    &["at", "christmas"],
    &["at", "easter"],
    // Round 27 (R27-B): "above all else" joins as
    // a phrase unit — the synthesis amends M7's
    // disposition: this is M7's first plausible
    // member (a one-word extension of the shipped
    // "above all"); M7's remaining merged forms
    // stay recorded.
    &["above", "all", "else"],
];

/// The token span of a listed phrase unit standing
/// at `i` in `words`, if one does (round 24,
/// R24-2): the unit's words must stand consecutively
/// as a whole phrase (empty cleaned tokens — a
/// boundary token the shared cleaning reduced to
/// nothing — may stand between them, as they may
/// stand anywhere in a region). The longest unit
/// matching at `i` wins, so "care of the" is taken
/// as one unit, not as "care of" plus the article.
pub(super) fn bare_phrase_unit_span(words: &[String], i: usize) -> Option<usize> {
    // Round 32 (R32-C, F3): a unit standing at `i`
    // must open on the first word that is something
    // at or after `i` — the match below skips the
    // tokens that clean to nothing before comparing
    // its first word — so a unit whose first word is
    // not that word cannot match here, and when no
    // word that is something stands at or after `i`
    // no unit can match at all. The shipped code
    // tried every listed unit at every position,
    // and each attempt rescanned the whole run of
    // nothing-tokens standing there: a region of
    // thousands of nothings (a de-grouped header
    // core is mostly the composition's own ";"
    // markers) cost quadratic time in exactly this
    // test, once per cancelled value judged against
    // it. The filter states the match's own first
    // step once; every unit it passes is matched
    // word for word as before, and the span it
    // returns is the span the unfiltered match
    // returned.
    let mut first = i;
    while words.get(first).is_some_and(|w| w.is_empty()) {
        first += 1;
    }
    let first_word = words.get(first)?;
    let mut best: Option<usize> = None;
    for unit in BARE_PHRASE_UNITS {
        if unit.first().copied() != Some(first_word.as_str()) {
            continue;
        }
        let mut pos = i;
        let mut matched = true;
        for want in unit.iter() {
            while words.get(pos).is_some_and(|w| w.is_empty()) {
                pos += 1;
            }
            if words.get(pos).map(String::as_str) != Some(*want) {
                matched = false;
                break;
            }
            pos += 1;
        }
        if matched {
            let span = pos - i;
            if best.map_or(true, |b| span > b) {
                best = Some(span);
            }
        }
    }
    best
}

/// Whether the occurrence at `idx` in `positional`
/// is a BARE restatement of its date (round 19,
/// R19-1): the occurrence's own region of the line —
/// the words standing after the previous occurrence
/// and before it, and the words standing after it up
/// to the next occurrence (or the end) — carries,
/// besides the date span itself, nothing but framing
/// words, filler words, and governor phrases. An
/// occurrence heading its own event clause — a
/// label, subject, or object beyond the framing,
/// before the date ("early service November 1",
/// "choir November 1") or after it ("November 1,
/// choir") — is not bare, wherever it stands.
/// Which tokens stand inside a bracket span of
/// two or more tokens (round 25, R25-A, M4 part
/// 1): a span "[our choir]" is no single token's
/// bracketed token, but in the bare partition the
/// whole span cleans to garnish exactly as a
/// wholly bracketed token ("[choir]") already
/// does. A one-token span is not marked here —
/// `group_token` already cleans it to nothing.
pub(super) fn bracket_span_mask(raw_tokens: &[&str]) -> Vec<bool> {
    let mut mask = vec![false; raw_tokens.len()];
    let mut depth: i32 = 0;
    let mut span_start: Option<usize> = None;
    for (i, raw) in raw_tokens.iter().enumerate() {
        let opens = raw.chars().filter(|c| *c == '[' || *c == '{').count() as i32;
        let closes = raw.chars().filter(|c| *c == ']' || *c == '}').count() as i32;
        if depth == 0 && opens > closes {
            span_start = Some(i);
        }
        depth += opens - closes;
        if depth <= 0 {
            depth = 0;
            if let Some(s) = span_start.take() {
                if i > s {
                    for m in &mut mask[s..=i] {
                        *m = true;
                    }
                }
            }
        }
    }
    // Round 26 (R26-C, M2 = review R26-6): a span
    // that never closes is a span to the end of the
    // token run — before this round the mask marked
    // only closed spans, so a date standing inside
    // an unclosed span ("(November 1, cancelled,
    // [our choir November 1)") was judged outside
    // any span and asserted where its closed twin
    // is a bracketed restatement that reads
    // nothing. The alone-unread limitation stands:
    // a line carrying nothing but the unclosed span
    // still reads nothing, exactly as before — the
    // mask feeds judgments about governed lines,
    // and a lone span has no governor. (Round 36,
    // R36-F, F8: the mask itself is unchanged, but
    // the plain date readers now space a stray
    // mark standing at the head of the line before
    // any mask is consulted — `stray_mark_spaced`
    // — so on those readers the lone span reads as
    // its paren twin; the limitation survives here,
    // where a sibling stands before the mark.)
    if let Some(s) = span_start {
        for m in &mut mask[s..] {
            *m = true;
        }
    }
    mask
}

/// Whether the token at `t` in `tokens` is the
/// weekday word of a live weekday signal in
/// `positional` (round 28, R28-C, F3): a bare
/// weekday signal standing on the token itself
/// (a one-token span), or a "next Sunday" / "this
/// Sunday" relative phrase whose span covers the
/// token. A framed date whose span covers the
/// token is NOT a weekday signal — a framed
/// date's own leading weekday word included —
/// and neither is a Sunday-after phrase standing
/// on the token: both carry spans longer than one
/// token that start on the token itself, not on a
/// relative word.
pub(super) fn weekday_signal_word(
    positional: &[(SimpleDate, usize, usize)],
    tokens: &[String],
    t: usize,
) -> bool {
    if t >= tokens.len() || weekday_word(&tokens[t]).is_none() {
        return false;
    }
    positional.iter().any(|(_, s, u)| {
        *s <= t && t < s + u && (*u == 1 || tokens[*s] == "next" || tokens[*s] == "this")
    })
}

/// The inventory's head-noun classes for the
/// determiner rule in `occurrence_is_bare` (round
/// 28, R28-C, F6): the words of `bare_filler_word`
/// that can stand as a noun phrase's head noun —
/// the standing time nouns and their round-27
/// time-companion kin (B1), the lone titles (B3),
/// the directions (B4), and the ordinals (the
/// standing first / second / third and B5's
/// remainder, "last" included). The inventory's
/// function classes are NOT head nouns: a
/// determiner or quantifier standing after "the"
/// ("the same", "the whole") heads no noun
/// phrase, the sign-off words (B2) and the
/// pronoun tail (B6) name no referent, and the
/// cardinals name a count, not a referent — all
/// keep their standing bare readings.
pub(super) fn inventory_head_noun(w: &str) -> bool {
    matches!(
        w,
        // The standing time nouns and the B1
        // time companions.
        "day"
            | "days"
            | "week"
            | "weeks"
            | "month"
            | "months"
            | "year"
            | "years"
            | "hour"
            | "hours"
            | "time"
            | "noon"
            | "midnight"
            | "tonight"
            | "morning"
            | "afternoon"
            | "evening"
            | "night"
            | "weekend"
            | "dawn"
            | "dusk"
            | "sunrise"
            | "sunset"
            | "daytime"
            | "nighttime"
            | "weekday"
            | "minute"
            | "moment"
            | "lunchtime"
            | "bedtime"
            | "daybreak"
            | "nightfall"
            | "twilight"
            | "mealtime"
            | "teatime"
            | "suppertime"
            | "dinnertime"
            | "spring"
            | "summer"
            | "autumn"
            | "fall"
            | "winter"
            | "season"
            | "midday"
            | "midweek"
            | "nightly"
            | "hourly"
            | "yearly"
            // The B3 lone titles.
            | "pastor"
            | "reverend"
            | "father"
            | "brother"
            | "sister"
            | "deacon"
            | "elder"
            | "bishop"
            | "chaplain"
            | "rabbi"
            | "priest"
            | "minister"
            | "missionary"
            | "professor"
            | "doctor"
            | "captain"
            | "major"
            | "colonel"
            | "general"
            | "sir"
            | "madam"
            | "madame"
            | "lord"
            | "lady"
            | "king"
            | "queen"
            | "prince"
            | "princess"
            | "saint"
            | "pope"
            | "president"
            | "chairman"
            | "secretary"
            | "treasurer"
            | "director"
            | "manager"
            | "coach"
            | "teacher"
            | "nurse"
            | "officer"
            | "judge"
            | "mayor"
            | "senator"
            | "miss"
            | "hon"
            | "esq"
            // The B4 directions.
            | "north"
            | "south"
            | "east"
            | "west"
            | "northeast"
            | "northwest"
            | "southeast"
            | "southwest"
            | "uptown"
            | "downtown"
            | "midtown"
            // The ordinals: the standing first /
            // second / third, B5's remainder, and
            // "last".
            | "first"
            | "second"
            | "third"
            | "fourth"
            | "fifth"
            | "sixth"
            | "seventh"
            | "eighth"
            | "ninth"
            | "tenth"
            | "eleventh"
            | "twelfth"
            | "thirteenth"
            | "fourteenth"
            | "fifteenth"
            | "sixteenth"
            | "seventeenth"
            | "eighteenth"
            | "nineteenth"
            | "twentieth"
            | "thirtieth"
            | "fortieth"
            | "fiftieth"
            | "hundredth"
            | "last"
    )
}

/// The preposition words of the bare inventory
/// (round 28, R28-C, F6): the families
/// `bare_filler_word` lists across rounds 20–23,
/// named here for the determiner rule's lone-
/// preposition block.
pub(super) fn bare_preposition_word(w: &str) -> bool {
    matches!(
        w,
        "on" | "for"
            | "of"
            | "to"
            | "from"
            | "at"
            | "by"
            | "in"
            | "until"
            | "before"
            | "after"
            | "through"
            | "circa"
            | "about"
            | "around"
            | "near"
            | "with"
            | "during"
            | "since"
            | "within"
            | "without"
            | "upon"
            | "unto"
            | "into"
            | "over"
            | "under"
            | "between"
            | "among"
            | "against"
            | "per"
            | "via"
            | "onto"
            | "toward"
            | "towards"
            | "across"
            | "behind"
            | "beyond"
            | "beside"
            | "despite"
            | "amid"
            | "past"
            | "off"
            | "besides"
            | "concerning"
            | "regarding"
            | "atop"
            | "amidst"
            | "amongst"
            | "thru"
            | "versus"
            | "vs"
    )
}

/// The index of the nearest word before `pos`
/// on the same side (at or after `side_start`)
/// that is something, if one stands there.
pub(super) fn prev_word_pos(words: &[String], side_start: usize, pos: usize) -> Option<usize> {
    (side_start..pos).rev().find(|p| !words[*p].is_empty())
}

/// Whether a word may stand as a bare-class
/// MODIFIER between a determiner and its head
/// noun (round 29, R29-C, F6 arm 3): the
/// determiner / quantifier class of the bare
/// inventory, the determiner-adjectives, and
/// the prenominal ordinal / sequence words.
/// Such a word modifies the head; it does not
/// head the phrase itself, so it cannot defeat
/// a determiner standing before it — while
/// standing AS a head ("the same", with no noun
/// after it) it keeps its shipped bare reading,
/// because this test is only ever read looking
/// back from a head noun.
pub(super) fn determiner_modifier_word(w: &str) -> bool {
    matches!(
        w,
        "same"
            | "other"
            | "another"
            | "such"
            | "own"
            | "each"
            | "every"
            | "either"
            | "neither"
            | "both"
            | "all"
            | "any"
            | "some"
            | "much"
            | "many"
            | "few"
            | "several"
            | "enough"
            | "most"
            | "more"
            | "less"
            | "least"
            | "little"
            | "various"
            | "numerous"
            | "countless"
            | "fewer"
            | "one"
            // The determiner-adjectives (R26-B, M6).
            | "whole"
            | "entire"
            | "certain"
            | "particular"
            | "specific"
            | "additional"
            | "extra"
            | "further"
            | "single"
            | "sole"
            | "half"
            | "double"
            // The ordinals and the prenominal
            // sequence words.
            | "first"
            | "second"
            | "third"
            | "fourth"
            | "fifth"
            | "sixth"
            | "seventh"
            | "eighth"
            | "ninth"
            | "tenth"
            | "eleventh"
            | "twelfth"
            | "thirteenth"
            | "fourteenth"
            | "fifteenth"
            | "sixteenth"
            | "seventeenth"
            | "eighteenth"
            | "nineteenth"
            | "twentieth"
            | "thirtieth"
            | "fortieth"
            | "fiftieth"
            | "hundredth"
            | "last"
            | "previous"
            | "preceding"
            | "next"
            | "following"
            | "coming"
    )
}

/// Whether a word can stand FIRST in a complex
/// preposition whose tail preposition stands
/// immediately after it (round 29, R29-C, F6
/// arm 1): a preposition word itself ("off of",
/// "in between"), a preposition-like word the
/// bare-preposition list does not carry ("as",
/// "inside", "outside", "along", "up"), or one
/// of the causal / exception / sequence words
/// that head a two-word complex preposition in
/// office prose ("because of", "apart from",
/// "aside from", "instead of", "except for",
/// "but for", "save for", "away from",
/// "next to", "due to", "thanks to", and kin).
/// A word outside this class standing before a
/// preposition — a governor participle
/// ("cancelled with …"), a conjunction ("and
/// with …"), a consumed unit ("back together
/// with …") — forms no preposition phrase with
/// it: the preposition after such a word stands
/// alone, exactly as the lone-preposition block
/// has always judged it.
pub(super) fn complex_preposition_starter(w: &str) -> bool {
    bare_preposition_word(w)
        || matches!(
            w,
            "as" | "inside"
                | "outside"
                | "along"
                | "up"
                | "because"
                | "apart"
                | "aside"
                | "instead"
                | "except"
                | "but"
                | "save"
                | "away"
                | "next"
                | "due"
                | "owing"
                | "thanks"
                | "prior"
                | "subsequent"
                | "contrary"
                | "close"
                | "ahead"
                | "opposite"
                | "regardless"
                | "irrespective"
                | "care"
        )
}

/// Whether a word can stand as the middle noun
/// of a three/four-word complex preposition —
/// preposition + noun + tail ("in spite of",
/// "in need of", "on behalf of"), or
/// preposition + determiner + noun + tail
/// ("at the time of", "in the event of", "for
/// the sake of"). The class is the nouns such
/// phrases are built from; the time-of-day and
/// season nouns are NOT in it — they head the
/// temporal phrases of the B1 ring ("in the
/// morning"), and a preposition standing after
/// one of those phrases is a lone preposition,
/// not this phrase's tail.
pub(super) fn complex_preposition_middle(w: &str) -> bool {
    matches!(
        w,
        "time"
            | "need"
            | "case"
            | "spite"
            | "behalf"
            | "front"
            | "back"
            | "top"
            | "way"
            | "means"
            | "regard"
            | "respect"
            | "reference"
            | "addition"
            | "connection"
            | "relation"
            | "response"
            | "accordance"
            | "compliance"
            | "consequence"
            | "consideration"
            | "contrast"
            | "excess"
            | "terms"
            | "line"
            | "place"
            | "exchange"
            | "favour"
            | "favor"
            | "account"
            | "reason"
            | "result"
            | "event"
            | "sake"
            | "light"
            | "view"
            | "lieu"
            | "care"
            | "charge"
            | "name"
            | "honor"
            | "honour"
            | "memory"
            | "presence"
            | "absence"
            | "company"
            | "course"
            | "midst"
            | "middle"
            | "end"
            | "side"
            | "heart"
            | "hope"
            | "fear"
            | "reply"
            | "opposed"
    )
}

/// Whether the preposition at `tail` in `words`
/// is the tail of a complex preposition, judged
/// as a phrase SHAPE (round 29, R29-C, F6 arm 1),
/// not by `BARE_PHRASE_UNITS` membership, which
/// stays at 166. The shapes are the two-word
/// shape (a complex-preposition starter
/// immediately before the tail), the three-word
/// shape (starter plus middle noun plus tail),
/// and the four-word shape (starter plus
/// determiner plus middle noun plus tail). Unit
/// membership is a separate, older reach into
/// the same reading and is judged by the caller.
pub(super) fn complex_preposition_tail(words: &[String], side_start: usize, tail: usize) -> bool {
    let Some(a) = prev_word_pos(words, side_start, tail) else {
        return false;
    };
    if complex_preposition_starter(&words[a]) {
        return true;
    }
    if !complex_preposition_middle(&words[a]) {
        return false;
    }
    let Some(b) = prev_word_pos(words, side_start, a) else {
        return false;
    };
    if complex_preposition_starter(&words[b]) {
        return true;
    }
    if !matches!(words[b].as_str(), "the" | "a" | "an") {
        return false;
    }
    let Some(c) = prev_word_pos(words, side_start, b) else {
        return false;
    };
    complex_preposition_starter(&words[c])
}

/// Whether the word at `i` in an occurrence's
/// region words stands as the head noun of a
/// determiner-headed noun phrase that reads as
/// content (round 28, R28-C, F6; the reach
/// restated round 29, R29-C, F6): the determiner
/// standing before it on its own side of the
/// date — the region's before and after sides
/// are separate sequences, and a noun phrase
/// cannot span the date — with any bare-class
/// modifier words between determiner and head
/// skipped (the head governs the reading), is a
/// definite determiner, AND that determiner is
/// not itself the object of a lone bare
/// preposition. The definite determiners are
/// "the" / "this" / "that" (round 29): "the"
/// heads a reference standing first on its side
/// or after any non-preposition word, as shipped;
/// a demonstrative heads one only when reached
/// through a complex preposition's tail —
/// a demonstrative standing at its side's head
/// is the deictic phrase the bare inventory
/// already judges ("this morning", the B1 ring,
/// keeps its standing governed reading). A
/// possessive standing before a title is R27-B's
/// recorded partition correction ("your pastor"
/// partitions word by word and stays governed),
/// and an indefinite article heads no definite
/// reference (the standing "a week ago" phrases
/// stay governed). The lone-preposition block is
/// the "in the morning" line and stands as
/// shipped (round 29 adjudication): a preposition
/// standing alone before the determiner keeps
/// its standing bare reading, while a determiner
/// reached through a complex preposition's tail
/// — a consumed unit ("in spite of the pastor",
/// "thanks to the pastor") or any phrase of that
/// shape, unit member or not ("because of the
/// pastor", "in need of the pastor", "at the
/// time of the pastor") — or "the" standing
/// first on its side heads a complete definite
/// reference and reads as content.
pub(super) fn determiner_headed_content(
    words: &[String],
    i: usize,
    after_start: usize,
    unit_spans: &[(usize, usize)],
) -> bool {
    let side_start = if i < after_start { 0 } else { after_start };
    // The determiner standing before the head:
    // bare-class modifiers between it and the
    // head are skipped — the head governs.
    let mut cursor = i;
    let p = loop {
        let Some(p) = prev_word_pos(words, side_start, cursor) else {
            return false;
        };
        if determiner_modifier_word(&words[p]) {
            cursor = p;
            continue;
        }
        break p;
    };
    let demonstrative = match words[p].as_str() {
        "the" => false,
        "this" | "that" => true,
        _ => return false,
    };
    let Some(q) = prev_word_pos(words, side_start, p) else {
        // Standing first on its side: "the"
        // heads the reference, as shipped; a
        // demonstrative standing first is the
        // deictic phrase and does not.
        return !demonstrative;
    };
    if !bare_preposition_word(&words[q]) {
        // After a non-preposition word the same
        // line holds: "the" heads the reference;
        // a demonstrative there is a deictic
        // standing in its own phrase, not a
        // reference reached through a tail.
        return !demonstrative;
    }
    // The determiner is the object of the
    // preposition at `q`: it heads the reference
    // exactly when that preposition is a complex
    // preposition's tail — a consumed unit's
    // member, or the tail of a phrase of the
    // complex shape. A lone bare preposition's
    // object stays excluded, as shipped.
    unit_spans.iter().any(|(s, e)| *s <= q && q < *e)
        || complex_preposition_tail(words, side_start, q)
}

/// The bare test, with the token run's bracket-span
/// mask computed by the caller (round 32, R32-C,
/// F3): the mask is a property of the whole token
/// run, and the by-date passes judge every
/// occurrence of a run against it — computing it
/// inside the test rebuilt it once per occurrence,
/// so a run holding thousands of occurrences (one
/// big group, a line of bracketed dates) paid for
/// the whole run once per occurrence. Every caller
/// now computes the mask once for its run and
/// judges through here. No judgment changes.
pub(super) fn occurrence_is_bare_masked(
    tokens: &[String],
    positional: &[(SimpleDate, usize, usize)],
    idx: usize,
    span_mask: &[bool],
) -> bool {
    let (_, start, used) = positional[idx];
    let region_start = if idx == 0 {
        0
    } else {
        positional[idx - 1].1 + positional[idx - 1].2
    };
    let region_end = positional
        .get(idx + 1)
        .map(|(_, s, _)| *s)
        .unwrap_or(tokens.len());
    // Round 20 (R20-2): a passive predicate
    // construction standing in the region — an
    // auxiliary with a governor participle as its
    // predicate, "it was moved", "it was
    // rescheduled", "it was cancelled" — is NOT
    // bare, however the region's words classify one
    // by one. Such a construction heads its own
    // event clause (round 19, R19-1's rule), and the
    // spent-predicate machinery already asserts its
    // date standing alone; the bare test must not
    // consume the auxiliary and the participle as
    // governor-phrase words and suppress that
    // clause's date under a sibling's governance.
    // The two sides of the date are judged
    // separately: a predicate cannot span the date
    // itself, so an auxiliary before the date and a
    // governor after it ("was November 1,
    // cancelled") form no construction here.
    let before_words = &tokens[region_start..start.min(tokens.len())];
    let after = (start + used).min(tokens.len());
    let after_words = &tokens[after..region_end.min(tokens.len())];
    // Round 21 (R21-5): the after side is judged by
    // the bounded rule — a passive "for" lead-in to
    // the next occurrence is that occurrence's
    // construction, not this one's (see
    // `after_region_has_own_passive_predicate`).
    let has_next = idx + 1 < positional.len();
    if region_has_passive_predicate(before_words)
        || after_region_has_own_passive_predicate(after_words, has_next)
    {
        return false;
    }
    let mut words: Vec<String> = Vec::new();
    words.extend_from_slice(before_words);
    // Round 21 (R21-8): the trailing discourse
    // word "thanks" is bare in the TRAILING slot
    // only — the words standing after the date.
    // In head position "thanks" is a content
    // label by the R20-3 adjudication and a
    // restatement wearing it asserts, as
    // "(thanks November 1)" always has. Round 22
    // (R22-8): "too" is bare in both slots (it
    // joins `bare_filler_word` above), so only
    // "thanks" still needs this trailing rule.
    let after_start = words.len();
    words.extend_from_slice(after_words);
    // Round 25 (R25-A, M4 part 1): the bracket-span
    // garnish flags for the region's words, aligned
    // with `words` — the before-date words first,
    // then the after-date words, each mapped back
    // to its token index for the span mask.
    let mut bracket_garnish: Vec<bool> = Vec::with_capacity(words.len());
    bracket_garnish.extend(
        (region_start..start.min(tokens.len())).map(|k| span_mask.get(k).copied().unwrap_or(false)),
    );
    bracket_garnish.extend(
        (after..region_end.min(tokens.len())).map(|k| span_mask.get(k).copied().unwrap_or(false)),
    );
    // Round 28 (R28-C, F6): the phrase-unit spans
    // this partition consumes, in region-word
    // coordinates — the determiner rule below
    // reads whether the word before an article
    // stood inside one (a complex preposition's
    // tail) or alone (a lone bare preposition).
    let mut unit_spans: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        // Round 24 (R24-2): a listed phrase unit
        // standing at this word is consumed as a
        // unit, before any member is judged alone —
        // "per" is bare word by word but "se" is
        // not, and "together" / "at" are bare but
        // "last" is not, so a member-by-member pass
        // strands both units on their last word.
        // Round 32 (R32-C, F3): the unit test is
        // asked only at a word that is something.
        // Asked at a nothing position, a matching
        // unit is either refused by the no-span
        // rule below (its words stand past the
        // date boundary the position precedes) or
        // consumed — and in the consumed case the
        // same unit, with the same span end, is
        // consumed when the walk reaches its head
        // word: the nothing positions before it are
        // skipped by the garnish step below either
        // way, and the one reader of `unit_spans`
        // (the determiner rule) queries it only at
        // word positions, where a span opening on
        // a nothing and a span opening on its head
        // word cover the same words. Asking at
        // every nothing position instead rescanned
        // the nothing run once per position — the
        // de-grouped core's ";" runs again.
        if !words[i].is_empty() {
            if let Some(span) = bare_phrase_unit_span(&words, i) {
                // Round 25 (R25-A, M7 = review R25-5): a
                // unit may not span the date itself —
                // the before-date and after-date words
                // are judged as separate sequences, the
                // same no-span rule the passive-
                // predicate test above already follows.
                // "(gone November 1 tomorrow)" and
                // "(care November 1 of)" are not the
                // units "gone tomorrow" / "care of".
                let crosses_date =
                    i < after_start && (after_start..i + span).any(|p| !words[p].is_empty());
                if !crosses_date {
                    // Round 28 (R28-C, F3 = review
                    // R28 MAJOR-2): a bare unit's own
                    // weekday signal is part of the
                    // bare span. The unit is judged as
                    // written in the token stream, not
                    // only by the part of it standing
                    // inside this region: a region ends
                    // at the next occurrence, so when
                    // the unit's own weekday word IS
                    // that next occurrence — a live
                    // signal of this very line or group
                    // (see `weekday_signal_word`) — the
                    // region's partition consumes only
                    // the unit's head ("see you") and
                    // the span as written still carries
                    // the signal. Such a span is not
                    // bare material for this occurrence
                    // and the region is not bare: the
                    // shipped unit &["see", "you",
                    // "sunday"] made the restated
                    // November 1 in "November 1, 2026,
                    // cancelled, November 1, 2026 see you
                    // sunday" read bare, while the
                    // Sunday inside the unit still
                    // spawned its weekday signal and the
                    // phantom October 18 won the
                    // segment. Judged with the unit
                    // standing, the occurrence asserts
                    // and the signal is named beside it,
                    // as the line read before the unit
                    // shipped. The unit's membership
                    // stands and its group readings
                    // stand (the group twin offers its
                    // alternatives by its two signals,
                    // not by bareness); weekday phrases
                    // that partition word by word —
                    // "coming sunday", "last sunday" —
                    // spawn their signals without any
                    // unit and keep their pinned
                    // readings, untouched by this rule.
                    let unit_token_start = if i < after_start {
                        region_start + i
                    } else {
                        after + (i - after_start)
                    };
                    let written_span = bare_phrase_unit_span(tokens, unit_token_start).unwrap_or(0);
                    let unit_has_live_weekday = (unit_token_start..unit_token_start + written_span)
                        .any(|t| weekday_signal_word(positional, tokens, t));
                    if unit_has_live_weekday {
                        return false;
                    }
                    unit_spans.push((i, i + span));
                    i += span;
                    continue;
                }
            }
        }
        // Round 28 (R28-C, F6 = red team RT28-B1,
        // coordinator-adjudicated): a determiner-
        // headed group whose head noun is an
        // inventory word reads as CONTENT in this
        // test. B3 joined LONE titles — "(pastor
        // November 1)" governs, as shipped — and
        // title+name pairs stayed out because a
        // title standing with its name is content.
        // A determiner-headed group, "the pastor",
        // is a complete definite reference, the
        // same grammatical object as "the choir",
        // so an inventory head noun standing under
        // the definite article is not consumed as
        // bare here (see `determiner_headed_content`
        // for the line's exact shape — the definite
        // determiners, restated round 29 (R29-C),
        // and never under a lone bare
        // preposition): "(in spite of the pastor,
        // November 1)" and "(thanks to the pastor,
        // November 1)" assert beside a cancelled
        // sibling, as their choir twins do, for
        // every head-noun class (titles, time
        // nouns, directions, ordinals). A weekday
        // word is never a head noun for this rule:
        // it is a signal word, not an inventory
        // noun (R28-C, F3's family), and bracket
        // garnish keeps its standing garnish
        // reading.
        if !words[i].is_empty()
            && words[i].chars().count() > 1
            && !bracket_garnish.get(i).copied().unwrap_or(false)
            && inventory_head_noun(&words[i])
            && weekday_word(&words[i]).is_none()
            && determiner_headed_content(&words, i, after_start, &unit_spans)
        {
            return false;
        }
        // Round 25 (R25-A, M4 part 1): a cleaned
        // single-character token is garnish in the
        // bare partition ("(November 1 1)", "(x
        // November 1)" resurrect nowhere), and a
        // word standing inside a multi-token
        // bracket span is garnish as a wholly
        // bracketed token already is ("(November 1
        // [our choir])").
        if words[i].is_empty()
            || words[i].chars().count() == 1
            || bracket_garnish.get(i).copied().unwrap_or(false)
            || bare_filler_word(&words[i])
            || (i >= after_start && matches!(words[i].as_str(), "too" | "thanks"))
        {
            i += 1;
            continue;
        }
        // A governor phrase standing in the region —
        // the families `is_governor_phrase` knows,
        // longest first — is the governance itself,
        // not event content: "cancelled, November 1"
        // restates its date bare.
        // Round 28 (R28-A, F2): the phrase is read
        // in WORDS (see `word_phrase`) — the region
        // is the run's words over cleaned tokens, so
        // a phrase wearing a nothing between its
        // words consumes its words, and the walk
        // steps to just past its last word.
        let mut consumed_end = 0usize;
        for len in (1..=3usize).rev() {
            if let Some((phrase, end)) = word_phrase(&words, i, len) {
                if is_governor_phrase(&phrase) {
                    consumed_end = end;
                    break;
                }
            }
        }
        if consumed_end > 0 {
            i = consumed_end;
            continue;
        }
        return false;
    }
    true
}
