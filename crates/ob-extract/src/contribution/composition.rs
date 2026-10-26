//! Group and composition analysis: what a whole
//! line, read together, contributes — parenthesized
//! groups, plain and governed compositions, second
//! dates, and the date line's full reading.

use super::dates::*;
use super::grammar::*;
use super::segments::*;
use super::*;

/// What one parenthesized group offers (round 11,
/// R11-4/R11-6/R11-8): at most one contributed date,
/// and — when the group offers two or more dates
/// instead — the offered dates as named alternatives,
/// which the account names on the unarmed second-date
/// note and which never stand as the group's date.
pub(super) struct GroupAnalysis {
    pub(super) contributed: Option<SimpleDate>,
    pub(super) alternatives: Vec<SimpleDate>,
    // The date values this group governs directly,
    // in one of its own occurrences (round 19,
    // R19-2): the line-level pass over a line's
    // trailing groups reads these beside each
    // group's contribution, so a bare restatement
    // standing in a sibling group stays governed.
    pub(super) governed_values: Vec<SimpleDate>,
    // Whether the contributed date stands only on
    // bare restatements of it (round 19, R19-1/R19-2):
    // every contributing occurrence is bare and
    // none is a target claim's last target. A
    // contribution headed by its own event clause,
    // or asserted by a claim, is never suppressed
    // by governance standing in another group.
    pub(super) contributed_bare: bool,
}

pub(super) fn analyze_group(inner: &str, today: SimpleDate, anchor: SimpleDate) -> GroupAnalysis {
    // Round 12 (R12-7b): the content reading sees
    // through balanced inner parentheses — the inner
    // content is the group's content for date-reading
    // purposes, so "((November 1))" and "(note
    // (November 1))" read the date a flat group
    // reads, and a nested governor ("(note (not
    // November 1))") still governs it.
    let unwrapped = inner.replace(['(', ')'], " ");
    if let Some(d) = parse_flexible_date(unwrapped.trim(), today) {
        // The group is the date and its framing
        // alone — a bare restatement by construction
        // (round 19, R19-2), governing nothing.
        let analysis = GroupAnalysis {
            contributed: Some(d),
            alternatives: Vec::new(),
            governed_values: Vec::new(),
            contributed_bare: true,
        };
        return analysis;
    }
    let raw_tokens: Vec<&str> = unwrapped.split_whitespace().collect();
    let tokens: Vec<String> = segment_tokens(&raw_tokens);
    // Round 18 (R18-6): the group's signals are the
    // plain line's signal set — framed dates and
    // relative phrases alike, in written order — so
    // "(November 1, cancelled, next Sunday)" judges
    // its relative replacement exactly as its plain
    // twin does, where the relative used to not be a
    // signal in a group at all and the group read
    // nothing. Anchor rule (round 19, R19-3): a
    // group resolves ALL its relative signals
    // against the parse anchor, relative-only groups
    // exactly as mixed ones — "(next Sunday)" in a
    // file dated October 16, scanned October 23,
    // contributes October 25, its mixed twin's
    // reading. Round 18's mixed-only rule mirrored
    // plain lines, but a relative-only PLAIN line
    // asserts no hard date while a relative-only
    // group contributes one, so the asymmetry
    // resolved a late file's group a week late. A
    // Sunday-after phrase over a framed base carries
    // no anchor resolution (`anchored` is None) and
    // keeps its framed-base reading, as before.
    let signals = dates_inside_group_signals(&raw_tokens, &tokens, today, anchor);
    let positional: Vec<(SimpleDate, usize, usize)> = signals
        .iter()
        .map(|s| (s.anchored.unwrap_or(s.date), s.start, s.used))
        .collect();
    let mut qualified: Vec<(SimpleDate, usize, usize)> = Vec::new();
    // The qualified occurrences' own indexes, in
    // written order (round 18, R18-5): the
    // for-composition below judges the qualified set
    // by occurrence, not only by date value.
    let mut qualified_idx: Vec<usize> = Vec::new();
    // Round 15 (R15-6): dates disqualified ONLY by
    // a trailing phrase standing after the group's
    // last date — governed, but the line did name
    // them, and beside a date that stands they are
    // named on the second-date note like any other
    // distinct date the line named, never silently
    // dropped: "(November 1, November 8, drop it)"
    // asserts November 1 and names November 8.
    // Round 16 (R16-5): the naming extends to a date
    // governed BETWEEN dates, whatever family
    // governed it — a phrase standing between it and
    // the next date, in either position, or the
    // target rule of R16-1: "(November 1, usher
    // shift, November 8)" names November 1 beside
    // November 8's assertion, instead of dropping it
    // unnamed. A group-initial before-governor
    // ("(not November 1, November 8)") keeps the
    // standing silence: its governor stands before
    // every date, not between dates.
    let mut trailing_named: Vec<SimpleDate> = Vec::new();
    // Round 16 (R16-1): the target claim standing
    // for each occurrence, computed once.
    // Round 32 (R32-C): computed against one shared
    // scan of the group (`target_claims`).
    let claims: Vec<Option<TargetClaim>> = target_claims(&raw_tokens, &tokens, &positional);
    let any_target = claims.iter().any(|c| c.is_some());
    // Round 20 (R20-7): the base dates this group's
    // own Sunday-after phrases consume, read with
    // the standing phrase-run scan — a consumed
    // base is part of its phrase and is never
    // named as another date the group offered, on
    // any naming channel (round 19, R19-8's
    // exclusion, which held only on the plain-core
    // merge before this).
    let consumed_bases: Vec<SimpleDate> = {
        let mut bases: Vec<SimpleDate> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if let Some((base, end)) = sunday_after_phrase_run(&raw_tokens, &tokens, i, today) {
                if !bases.contains(&base) {
                    bases.push(base);
                }
                i = end.max(i + 1);
                continue;
            }
            i += 1;
        }
        bases
    };
    // Round 28 (R28-B, F5 tail = red team RT28-W4):
    // the resolutions of bare Sundays standing as
    // PPs inside the destination stack ("… from
    // Kent to the hall on Sunday to November 8"),
    // read by the naming-only consultation
    // (`weekday_consumed_in_destination_stack`).
    // Such a Sunday's occurrence bounds the
    // target's governor window — the target asserts
    // exactly as it does today — but the Sunday is
    // the stack's own material, and its resolution
    // is named nowhere beside that assertion: the
    // from and accompaniment twins of the shape
    // consume both Sundays outright, and this shape
    // silences the naming the same way. Excluded by
    // date value, as `consumed_bases` above is.
    // Round 29 (R29-B, F7 = red team RT29-N1,
    // review MINOR-2): the consultation covers
    // every Sunday the stack consumed, in every
    // position the stack carries one — a Sunday
    // standing AFTER the claim's target in the
    // trailing orders (the crossing's trailing
    // arm now carries the naming reading), and a
    // RELATIVE-phrase Sunday standing in the
    // stack's second PP ("… to the hall on next
    // Sunday to November 8"), whose positional
    // entry starts at its relative word: the
    // consultation is asked at the entry's Sunday
    // word, the last word of its span, exactly as
    // a bare entry is asked at its only word.
    let stack_consumed: Vec<SimpleDate> = positional
        .iter()
        .filter_map(|(d, start, used)| {
            let sunday_at = if tokens.get(*start).map(String::as_str) == Some("sunday") {
                Some(*start)
            } else if matches!(
                tokens.get(*start).map(String::as_str),
                Some("next") | Some("this")
            ) && *used >= 1
                && tokens.get(start + used - 1).map(String::as_str) == Some("sunday")
            {
                Some(start + used - 1)
            } else {
                None
            };
            sunday_at
                .filter(|s| weekday_consumed_in_destination_stack(&raw_tokens, &tokens, *s, today))
                .map(|_| *d)
        })
        .collect();
    // Round 18 (R18-2): the per-occurrence judgments
    // are collected first and applied in a second
    // pass, because governance attaches to the DATE
    // VALUE, not the occurrence (see below). Each
    // entry: (before, after, target, superseded).
    let mut judgments: Vec<(bool, bool, bool, bool)> = Vec::with_capacity(positional.len());
    // Round 18 (R18-5/R18-9): each occurrence's
    // "for" standing, judged from its own window —
    // whether the date is a for-occasion (a passive
    // or active "for" zone ends its window), and
    // whether it stands as a prospective "use" date
    // ("use November 8" is the window's last word
    // before the date). The for-composition below
    // reads these beside the qualified set.
    let mut for_occasions: Vec<bool> = Vec::with_capacity(positional.len());
    let mut use_prospectives: Vec<bool> = Vec::with_capacity(positional.len());
    for (idx, entry) in positional.iter().enumerate() {
        let (d, start, used) = *entry;
        // Round 12 (R12-2): the governor window for
        // this occurrence starts after the previous
        // occurrence's end — a governor is spent on
        // the one date it governs, and a later date
        // in the group is judged on its own window.
        let prev_end = if idx == 0 {
            0
        } else {
            positional[idx - 1].1 + positional[idx - 1].2
        };
        let window_start = governor_window_start(&raw_tokens, &tokens, prev_end, start, idx > 0);
        let window = &tokens[window_start..start.min(tokens.len())];
        for_occasions.push(
            passive_for_zone_start(&tokens[..window_start], window).is_some()
                || active_for_zone_start(window).is_some(),
        );
        // Round 28 (R28-A, F2): the window's last
        // word is its last word that is something —
        // a nothing standing between "use" and its
        // date ("use \u{200b} November 8") leaves
        // "use" standing before the date, and the
        // split pair's first word is read the same
        // way; its second word is the next word that
        // is something after the date's span.
        use_prospectives.push(last_word(window) == Some("use"));
        let next_start = positional.get(idx + 1).map(|(_, s, _)| *s);
        // Round 14 (R14-1): the split phrasal form —
        // "put" standing last in the window and "off"
        // standing first after the date is the pair
        // "put off" wrapped around its date:
        // "(put November 1 off)" governs. Round 15
        // (R15-1): the same wrap for every phrasal
        // pair whose first word inflects — "(called
        // November 1 off)", "(put November 1
        // back)", "(push November 1 back)".
        let split_first = last_word(window);
        let split_second = next_nonempty(&tokens, start + used).map(|n| tokens[n].as_str());
        let split_phrasal = match (split_first, split_second) {
            (Some(first), Some(second)) => is_phrasal_governor_pair(first, second),
            _ => false,
        };
        // Round 16 (R16-1): a target-claimed date is
        // judged by the claim — the construction's
        // own words do not govern it — and any OTHER
        // distinct date in a group whose target is
        // claimed is the phrase's from-date: governed
        // by the phrase and named, wherever it stands
        // ("(it was moved to November 8, November 1)").
        let claim = &claims[idx];
        let before_disqualified = match claim {
            Some(claim) => {
                governed_before_target(window, window_start, claim)
                    || split_phrasal
                    || contradicted_before(&tokens, start, d)
            }
            None => {
                governed_before(&tokens[..window_start], window)
                    || split_phrasal
                    || contradicted_before(&tokens, start, d)
            }
        };
        // Round 23 (R23-5): a framed date consumed
        // as a subject PP's object is that phrase's
        // own material — it does not qualify as an
        // occurrence of its own, exactly as the
        // "from" twin's date is governed in its
        // window and named beside the assertion.
        let before_disqualified = before_disqualified
            || date_in_subject_pp(&tokens, start, used)
            // Round 24 (R24-7): a relative phrase
            // consumed as a subject PP's object is
            // disqualified the same way — it does not
            // qualify as an occurrence of its own, and
            // the naming below names its resolution
            // beside the assertion exactly as it
            // names the framed twin's PP object.
            || relative_in_subject_pp(&tokens, start);
        let after_disqualified = governed_after(&raw_tokens, &tokens, start + used, next_start);
        let target_disqualified = claim.is_none()
            && any_target
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| c.is_some() && positional[j].0 != d);
        // Round 17 (R17-1): the LAST target in a
        // claim run wins. An earlier claimed target
        // superseded by a later distinct claimed
        // target is governed like the from-date —
        // "(November 1, moved to November 8, then to
        // November 15)" asserts November 15, with
        // November 1 and November 8 both named —
        // instead of the run collapsing to no
        // assertion at all.
        let superseded_target = claim.is_some()
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| j > idx && c.is_some() && positional[j].0 != d);
        judgments.push((
            before_disqualified,
            after_disqualified,
            target_disqualified,
            superseded_target,
        ));
    }
    // Round 18 (R18-2): governance by date value, the
    // group surface of the plain rule in
    // `plain_composition`. A date governed in ANY
    // occurrence (a governor before or after that
    // occurrence, the split wrap, a contradicting
    // weekday) does not assert from another
    // occurrence: "(November 1, cancelled, November
    // 1)" reads nothing, where the restated second
    // standing used to assert the cancelled date on
    // its own clean window. The one override is a
    // target claim naming the date as the LAST
    // target: "(meeting November 1, cancelled, moved
    // to November 1)" asserts November 1 by the
    // claim, the governed first standing named by
    // its own judgment. A by-date disqualification
    // standing after the group's first occurrence is
    // named like a between-dates governance; the
    // governed occurrence's own naming rules above
    // are unchanged.
    // Round 19 (R19-1): the by-date step binds only
    // a BARE restatement (see `occurrence_is_bare`).
    // An occurrence heading its own event clause
    // asserts or is governed by its own occurrence's
    // judgment, independently of the same value
    // elsewhere in the group: "(early service
    // November 1, late service cancelled November 1)"
    // asserts November 1 for the early service — the
    // governed occurrence is the late service's, and
    // the early service's occurrence carries its own
    // event content. Round 18's witnesses are all
    // bare restatements and stay silent.
    let governed_values: Vec<SimpleDate> = positional
        .iter()
        .zip(judgments.iter())
        .filter(|(_, (before, after, _, _))| *before || *after)
        .map(|(entry, _)| entry.0)
        .collect();
    // Round 32 (R32-C, F3): the group's bracket-span
    // mask, computed once for the by-date pass and
    // the contributed-bare test below — the bare
    // test used to rebuild it once per occurrence.
    // The governed values stand in a set beside
    // their list for the same reason: the by-date
    // membership test scanned the list once per
    // occurrence.
    let span_mask = bracket_span_mask(&raw_tokens);
    let governed_set: std::collections::BTreeSet<SimpleDate> =
        governed_values.iter().copied().collect();
    for (idx, entry) in positional.iter().enumerate() {
        let (d, _, _) = *entry;
        let (before_disqualified, after_disqualified, target_disqualified, superseded_target) =
            judgments[idx];
        let is_last_target = claims[idx].is_some() && !superseded_target;
        // Round 20 (R20-10): a Sunday-after phrase
        // occurrence is never bound by value — an
        // ungoverned phrase salvages by its own
        // resolution whatever its base's form,
        // exactly as its framed-base twin does (the
        // framed twin's resolution differs from its
        // governed base, so it never needed the
        // exemption; a relative base's resolution
        // can coincide with the governed restated
        // value). A phrase governed in its own
        // occurrence still dies by that judgment
        // above, untouched.
        let is_sunday_after_phrase = tokens.get(positional[idx].1).map(String::as_str)
            == Some("sunday")
            && tokens.get(positional[idx].1 + 1).map(String::as_str) == Some("after");
        let by_date = !(before_disqualified || after_disqualified)
            && governed_set.contains(&d)
            && !is_last_target
            && !is_sunday_after_phrase
            && occurrence_is_bare_masked(&tokens, &positional, idx, &span_mask);
        if before_disqualified
            || after_disqualified
            || target_disqualified
            || superseded_target
            || by_date
        {
            let next_start = positional.get(idx + 1).map(|(_, s, _)| *s);
            let named = target_disqualified
                || superseded_target
                || (after_disqualified && next_start.is_some())
                || (after_disqualified && !before_disqualified && next_start.is_none())
                || (before_disqualified && idx > 0 && next_start.is_some())
                || (by_date && idx > 0);
            if named && !trailing_named.contains(&d) {
                trailing_named.push(d);
            }
            continue;
        }
        qualified.push(*entry);
        qualified_idx.push(idx);
    }
    // Round 19 (R19-2): whether a date the group is
    // about to contribute stands only on bare
    // restatements of it — every qualified occurrence
    // of the value is bare, and none is a target
    // claim's last target (a claim asserts by its
    // own judgment and is never a restatement). The
    // line-level pass over trailing groups reads
    // this beside the sibling groups' governed
    // values.
    let contributed_bare = |d: SimpleDate| -> bool {
        let idxs: Vec<usize> = qualified_idx
            .iter()
            .copied()
            .filter(|i| positional[*i].0 == d)
            .collect();
        !idxs.is_empty()
            && idxs.iter().all(|i| {
                occurrence_is_bare_masked(&tokens, &positional, *i, &span_mask)
                    && !(claims[*i].is_some() && !judgments[*i].3)
            })
    };
    let partial = partial_or_alternative(&tokens, &qualified);
    // Round 32 (R32-C, F3): the distinct values and
    // the alternatives below are gathered against a
    // set standing beside the list, not by scanning
    // the list once per occurrence — in one big
    // group holding thousands of distinct dates that
    // scan was quadratic in the group's dates. The
    // lists themselves are built exactly as before,
    // first-written order, each value once.
    let mut distinct: Vec<SimpleDate> = Vec::new();
    let mut distinct_seen: std::collections::BTreeSet<SimpleDate> =
        std::collections::BTreeSet::new();
    for (d, _, _) in &qualified {
        if distinct_seen.insert(*d) {
            distinct.push(*d);
        }
    }
    // Round 18 (R18-5): a for-date enters the group's
    // composition as a signal like any other, instead
    // of collapsing the group to alternatives the
    // moment a second date stands beside it. Two
    // readings, both fenced to groups no target claim
    // already orders (a "then to …" chain reads by
    // its claim, as it always has):
    // - A prospective second date asserts over the
    //   for-date: "(it was moved for November 1, use
    //   November 8)" asserts November 8, the
    //   for-date named — "use" is the office's own
    //   assertion word, and the for-date is the
    //   occasion it replaces.
    // - Otherwise the for-date reads by primacy:
    //   "(it was scheduled for November 1 and
    //   November 8)" asserts November 1 with
    //   November 8 noted, in written order either
    //   way — the for construction names its
    //   occasion first, and the standing two-date
    //   rule (neither asserts, both offered) yields
    //   to it, as it yields to a claim.
    if distinct.len() >= 2 && !any_target && !qualified_idx.is_empty() {
        let use_asserts = qualified_idx
            .iter()
            .rev()
            .copied()
            .find(|i| use_prospectives[*i])
            .filter(|ui| {
                qualified_idx
                    .iter()
                    .any(|fi| *fi < *ui && for_occasions[*fi])
            });
        let for_outcome = match use_asserts {
            Some(ui) => Some(positional[ui].0),
            None if for_occasions[qualified_idx[0]] => Some(distinct[0]),
            None => None,
        };
        if let Some(contributed) = for_outcome {
            // Round 32 (R32-C): gathered against the
            // beside-set, as the alternatives below.
            let mut alternatives: Vec<SimpleDate> = Vec::new();
            let mut alternatives_seen: std::collections::BTreeSet<SimpleDate> =
                std::collections::BTreeSet::new();
            for d in &distinct {
                if *d != contributed && alternatives_seen.insert(*d) {
                    alternatives.push(*d);
                }
            }
            for d in &trailing_named {
                if *d != contributed
                    && !consumed_bases.contains(d)
                    && !stack_consumed.contains(d)
                    && alternatives_seen.insert(*d)
                {
                    alternatives.push(*d);
                }
            }
            if let Some(p) = partial {
                if p != contributed && alternatives_seen.insert(p) {
                    alternatives.push(p);
                }
            }
            let bare = contributed_bare(contributed);
            return GroupAnalysis {
                contributed: Some(contributed),
                alternatives,
                governed_values: governed_values.clone(),
                contributed_bare: bare,
            };
        }
    }
    if distinct.len() == 1 && partial.is_none() {
        let mut alternatives: Vec<SimpleDate> = Vec::new();
        for d in &trailing_named {
            if *d != distinct[0]
                && !consumed_bases.contains(d)
                && !stack_consumed.contains(d)
                && !alternatives.contains(d)
            {
                alternatives.push(*d);
            }
        }
        let bare = contributed_bare(distinct[0]);
        let analysis = GroupAnalysis {
            contributed: Some(distinct[0]),
            alternatives,
            governed_values: governed_values.clone(),
            contributed_bare: bare,
        };
        return analysis;
    }
    let mut alternatives: Vec<SimpleDate> = Vec::new();
    let mut alternatives_seen: std::collections::BTreeSet<SimpleDate> =
        std::collections::BTreeSet::new();
    for (d, _, _) in &qualified {
        if alternatives_seen.insert(*d) {
            alternatives.push(*d);
        }
    }
    // A trailing-governed date is named beside the
    // offered alternatives too, when the group
    // offers more than one date and asserts none —
    // but a lone governed date (distinct is empty)
    // keeps the standing silence: "(November 1,
    // cancelled)" names nothing anywhere.
    if distinct.len() >= 2 {
        for d in &trailing_named {
            if !consumed_bases.contains(d)
                && !stack_consumed.contains(d)
                && alternatives_seen.insert(*d)
            {
                alternatives.push(*d);
            }
        }
    }
    if let Some(p) = partial {
        if alternatives_seen.insert(p) {
            alternatives.push(p);
        }
    }
    GroupAnalysis {
        contributed: None,
        alternatives,
        governed_values: governed_values.clone(),
        contributed_bare: false,
    }
}

/// The date one parenthesized group contributes
/// (round 10, R10-5): a group that is itself a bare
/// date contributes that date, as it always has; a
/// group with words around a date contributes the
/// date it contains when it contains exactly one —
/// "correction: November 1", "use November 1 instead"
/// — and a group containing no date, or several,
/// contributes nothing.
///
/// Round 11 (R11-4/R11-8): a contained date governed
/// by a negation or an origin marker standing before
/// it, or contradicted by a weekday word standing
/// directly before it, does not contribute — the
/// analysis in `analyze_group` is the one rule, and
/// every consumer of group contributions reads it
/// through here.
///
/// Round 12 (R12-1/R12-2/R12-7): the governor
/// families are completed (skip / except / rather
/// than / instead of / cancelled / bare no / the
/// isn't family / never mind; previously / formerly
/// / used to be / was / old date), a governor scopes
/// to the one date in its own clause, a trailing
/// governor with no later date to govern
/// disqualifies the date it follows, and the
/// content reading sees through inner parentheses.
///
/// Round 13 (R13-1/R13-2): the families are
/// completed structurally — negation is the
/// "n't" contraction class (plus wont / cannot),
/// cancellation gains cancel / scratch / drop /
/// postpone / remove / hold off, labels gain
/// "wrong:" — and a clause break erases only a
/// spent governor: an unspent governor standing
/// as its clause's trailing element still governs
/// the first date of the next clause.
///
/// Round 14 (R14-1/R14-2/R14-3/R14-4/R14-5,
/// RT14-4): the governor machinery is completed —
/// the verb set gains put off (split form
/// included), delay, omit, disregard, forget, and
/// the cancellation verbs match by stem; the
/// trailing position draws from the same family
/// as the before position; "nope" joins the
/// negation family; hyphenated compounds split
/// for the governor test only; a possessive-
/// preceded "wont" is the noun and governs
/// nothing; and the unspent-governor reach-back
/// crosses consecutive filler-only clauses.
pub(super) fn group_date(inner: &str, today: SimpleDate, anchor: SimpleDate) -> Option<SimpleDate> {
    analyze_group(inner, today, anchor).contributed
}

/// A line's trailing parenthesized groups, in
/// written order, each with its analysis, under the
/// cross-group governance rule (round 19, R19-2):
/// governance by date value crosses a group boundary
/// exactly as it crosses a clause inside one group,
/// and binds exactly the same occurrences — a group
/// whose contribution stands only on BARE
/// restatements of a value that a SIBLING group on
/// the line governs directly contributes nothing
/// ("(November 1, cancelled) (November 1)" reads
/// nothing, in either group order, as the one-group
/// shapes do). A group heading its own event clause
/// asserts independently on either side of the
/// boundary ("(potluck November 1 cancelled)
/// (service November 1)" asserts November 1 for the
/// service — the potluck's cancellation governs the
/// potluck's occurrence, in its own group), and a
/// contribution a target claim asserts is never a
/// restatement (see `GroupAnalysis::contributed_bare`).
/// The line's plain core is not a sibling group
/// in THIS pass: a trailing group is judged on its
/// own beside the core here, and the core/group
/// composition (round 20, R20-1) applies the
/// core's governance to these analyses at its own
/// call sites (`governed_core_group_reading`,
/// `trailing_dates_after_core_governance`).
pub(super) fn trailing_group_analyses(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<(String, GroupAnalysis)> {
    // Round 40 (R40-B, F1): the splits are read
    // off the line's precomputed scan
    // (`LineScan::trailing_splits`) — the same
    // sequence the loop here used to walk one
    // split at a time, each split's edge facts
    // tabled once for the line.
    let scan = LineScan::new(text);
    let mut inners: Vec<String> = scan
        .trailing_splits()
        .iter()
        .map(|(_, inner)| inner.to_string())
        .collect();
    inners.reverse();
    let mut analyses: Vec<GroupAnalysis> = inners
        .iter()
        .map(|inner| analyze_group(inner, today, anchor))
        .collect();
    apply_cross_group_governance(&mut analyses);
    inners.into_iter().zip(analyses).collect()
}

/// The cross-group governance pass (round 19,
/// R19-2), shared by every group list a line is read
/// through: a group whose contribution stands only
/// on bare restatements of a value a SIBLING group
/// governs directly contributes nothing.
pub(super) fn apply_cross_group_governance(analyses: &mut [GroupAnalysis]) {
    // Round 31 (R31-C, F4): the sibling test is
    // counted once per value instead of rescanning
    // every sibling group for every group — the
    // pairwise scan cost quadratic time in the
    // number of groups on a header line (thousands
    // of repeated groups froze the parse). For each
    // date value, count the groups that govern it
    // directly (a group counts once per value,
    // however many governed occurrences it holds);
    // a bare contribution of `d` is then suppressed
    // exactly when a group OTHER than its own is
    // among those counted — the same test, with the
    // group's own governance subtracted back out.
    let mut governing_groups: std::collections::BTreeMap<SimpleDate, usize> =
        std::collections::BTreeMap::new();
    for a in analyses.iter() {
        let mut counted: Vec<SimpleDate> = Vec::new();
        for d in &a.governed_values {
            if !counted.contains(d) {
                counted.push(*d);
                *governing_groups.entry(*d).or_insert(0) += 1;
            }
        }
    }
    for a in analyses.iter_mut() {
        let Some(d) = a.contributed else {
            continue;
        };
        if !a.contributed_bare {
            continue;
        }
        let mut count = governing_groups.get(&d).copied().unwrap_or(0);
        if a.governed_values.contains(&d) {
            count -= 1;
        }
        if count > 0 {
            a.contributed = None;
        }
    }
}

/// Every parenthesized group standing in `text`, in
/// written order, each with its analysis, under the
/// same cross-group governance rule as the trailing
/// groups (round 21, R21-1/R21-3): the whole-line
/// composition reads a line's groups wherever they
/// stand — leading, trailing, or between plain
/// segments — so this is `trailing_group_analyses`
/// over the balanced spans instead of only the
/// trailing run.
pub(super) fn all_group_analyses(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<(String, GroupAnalysis)> {
    let inners: Vec<String> = balanced_spans(text)
        .iter()
        .map(|(start, end)| text[start + 1..end - 1].to_string())
        .collect();
    let mut analyses: Vec<GroupAnalysis> = inners
        .iter()
        .map(|inner| analyze_group(inner, today, anchor))
        .collect();
    apply_cross_group_governance(&mut analyses);
    inners.into_iter().zip(analyses).collect()
}

/// Every date a group standing anywhere in `text`
/// contributes, in written order: the whole-line
/// form of `trailing_parenthesized_dates`, for the
/// readers that remember a line's parenthesized
/// dates beside a relative phrase wherever the
/// groups stand (round 21, R21-1).
pub(super) fn all_parenthesized_dates(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    all_group_analyses(text, today, anchor)
        .iter()
        .filter_map(|(_, a)| a.contributed)
        .collect()
}

/// The last date a group standing anywhere in
/// `text` contributes, if one does: the whole-line
/// form of `trailing_parenthesized_date`.
pub(super) fn all_parenthesized_date(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<SimpleDate> {
    all_group_analyses(text, today, anchor)
        .iter()
        .rev()
        .find_map(|(_, a)| a.contributed)
}

pub(super) fn trailing_parenthesized_date(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<SimpleDate> {
    trailing_group_analyses(text, today, anchor)
        .iter()
        .rev()
        .find_map(|(_, a)| a.contributed)
}

/// Every date a trailing parenthesized group names,
/// in the order the groups stand in the line (round
/// 9, R9-10): groups are walked from the end like
/// `trailing_parenthesized_date`, a group that does
/// not read as a date is garnish and is skipped, and
/// the collected dates are put back in written order.
pub(super) fn trailing_parenthesized_dates(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    trailing_group_analyses(text, today, anchor)
        .iter()
        .filter_map(|(_, a)| a.contributed)
        .collect()
}

/// The trailing groups' contributed dates and
/// offered alternatives, read off the one
/// `trailing_group_analyses` computation (round
/// 40, R40-C, F2): `trailing_parenthesized_dates`
/// and `trailing_group_alternatives` are the two
/// projections of that computation — the dates
/// in the first's order, the alternatives in the
/// second's — and a caller needing both (the
/// `date_line` composition) pays the analyses
/// once per line instead of once per projection.
/// On a line of thousands of trailing groups the
/// duplicated pass was a full second walk of the
/// sequence readers over the same groups. (The
/// standalone alternatives projection was round
/// 11's `trailing_group_alternatives`, R11-6: a
/// group that offers two or more dates
/// contributes none of them, and these are named
/// on the line's second-date note instead.)
pub(super) fn trailing_dates_and_alternatives(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> (Vec<SimpleDate>, Vec<SimpleDate>) {
    let analyses = trailing_group_analyses(text, today, anchor);
    let dates: Vec<SimpleDate> = analyses.iter().filter_map(|(_, a)| a.contributed).collect();
    let mut alternatives: Vec<SimpleDate> = Vec::new();
    for (_, a) in &analyses {
        for d in &a.alternatives {
            if !alternatives.contains(d) {
                alternatives.push(*d);
            }
        }
    }
    (dates, alternatives)
}

/// Whether a group carries an actual target claim
/// for `read` (round 19, R19-9): either a target
/// claim judged inside the group itself — a target
/// phrase in the group claiming `read` through its
/// own "to" / "until", by the standing `target_claim`
/// test — or the chain's continuation link standing
/// in the group: a "to" / "until" before the `read`
/// occurrence with nothing between it and the date
/// but determiners and the link's own lead-ins
/// ("then", "and"), and no other date standing
/// between ("(then to November 15)", "(to November
/// 15)"). A bare "to" token anywhere in the group
/// is not a claim: "(November 15, note to self)" and
/// "(November 15, back to church)" wear their "to"
/// after the date, in garnish, and claim nothing.
pub(super) fn group_has_target_claim(
    inner: &str,
    read: SimpleDate,
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    let unwrapped = inner.replace(['(', ')'], " ");
    let raw_tokens: Vec<&str> = unwrapped.split_whitespace().collect();
    let tokens: Vec<String> = segment_tokens(&raw_tokens);
    let signals = dates_inside_group_signals(&raw_tokens, &tokens, today, anchor);
    let positional: Vec<(SimpleDate, usize, usize)> = signals
        .iter()
        .map(|s| (s.anchored.unwrap_or(s.date), s.start, s.used))
        .collect();
    // Round 32 (R32-C): the group's claims against
    // one shared scan, as in the composition passes.
    let scan = target_claim_scan(&raw_tokens, &tokens, &positional);
    for (idx, entry) in positional.iter().enumerate() {
        if entry.0 != read {
            continue;
        }
        if target_claim_in(&scan, &raw_tokens, &tokens, &positional, idx).is_some() {
            return true;
        }
        let start = entry.1;
        for to_pos in 0..start {
            if !matches!(tokens[to_pos].as_str(), "to" | "until") {
                continue;
            }
            if positional.iter().any(|(_, s, _)| *s > to_pos && *s < start) {
                continue;
            }
            if !tokens[to_pos + 1..start]
                .iter()
                .all(|w| matches!(w.as_str(), "the" | "a" | "an" | "then" | "and" | ""))
            {
                continue;
            }
            // Round 20 (R20-8): the link's "to" must
            // belong to a target construction — a
            // displacement/postpone phrase in the
            // group governs the link (a target phrase
            // standing before the "to" with nothing
            // between it and the "to" but its object
            // words: no governor, cancellation,
            // auxiliary, or weekday), or the link
            // directly continues one (the fragment
            // before the "to" is nothing but the
            // link's own lead-ins: "(then to
            // November 15)", "(to November 15)").
            // A garnish fragment wearing its own
            // content word before the "to" — "(note
            // to November 15)", "(back to November
            // 15)", "(return to November 15)" — is
            // prose, not a chain link, even when the
            // group carries a claim elsewhere.
            let phrase_governs_link = (0..to_pos).any(|ps| {
                (1..=2usize).any(|len| {
                    let Some(phrase) = tokens.get(ps..ps + len) else {
                        return false;
                    };
                    if ps + len > to_pos || !is_target_phrase(phrase) {
                        return false;
                    }
                    tokens[ps + len..to_pos].iter().all(|w| {
                        !direct_single_governor(w)
                            && !is_cancellation_verb(w)
                            && !AUXILIARY_WORDS.contains(&w.as_str())
                            && weekday_word(w).is_none()
                    })
                })
            });
            let directly_continues = tokens[..to_pos]
                .iter()
                .all(|w| matches!(w.as_str(), "the" | "a" | "an" | "then" | "and" | ""));
            if phrase_governs_link || directly_continues {
                return true;
            }
        }
    }
    false
}

/// The superseded targets of a target chain split
/// across trailing groups (round 18, R18-11): when
/// two or more trailing groups contribute dates and
/// the groups between them carry a target
/// construction — a target phrase with its "to" /
/// "until" in one group, a target claim in the
/// last (round 19, R19-9) — the chain reads across
/// the group boundary exactly as it reads inside
/// one group,
/// and every earlier contributing group's date that
/// differs from `read` is a superseded target: it
/// is named on the line's note, and it is not a
/// disagreeing garnish date, so it leaves the
/// parenthesized-mismatch comparison. A stack of
/// bare date groups carries no target construction
/// and keeps the standing mismatch reading; only
/// the chain shape is named here.
pub(super) fn chain_superseded_dates(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    read: SimpleDate,
) -> Vec<SimpleDate> {
    // The chain's groups are the line's trailing
    // groups under the same cross-group pass as
    // every other reader (round 19, R19-2): a chain
    // link asserts by its claim, so the pass leaves
    // chain shapes exactly as they stood.
    let entries: Vec<(String, Option<SimpleDate>)> = trailing_group_analyses(text, today, anchor)
        .into_iter()
        .map(|(inner, a)| (inner, a.contributed))
        .collect();
    chain_superseded_from_entries(&entries, read, today, anchor)
}

/// The chain test over a list of (group words,
/// contributed date) entries in written order —
/// `chain_superseded_dates` over a line's trailing
/// groups, and the whole-line composition (round 21)
/// over a line's groups wherever they stand. The
/// shape judged is the one stated there: a target
/// phrase with its "to" / "until" in some
/// contributing group, and an actual target claim
/// for `read` in the last contributing group.
pub(super) fn chain_superseded_from_entries(
    entries: &[(String, Option<SimpleDate>)],
    read: SimpleDate,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    let contributing: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, (_, d))| d.is_some())
        .map(|(i, _)| i)
        .collect();
    if contributing.len() < 2 {
        return Vec::new();
    }
    let last = *contributing.last().expect("two or more stand");
    if entries[last].1 != Some(read) {
        return Vec::new();
    }
    // The chain's shape, read across the groups: a
    // target phrase stands in some contributing
    // group with its "to" / "until" after it, and
    // the last contributing group carries an actual
    // target claim for the read date — the phrase
    // may stand in the first group while the last
    // group is only its continuation, "(moved to
    // November 8) (then to November 15)", which is
    // the witness shape. Round 19 (R19-9): the last
    // group's part used to ride on any "to" /
    // "until" token standing anywhere in it, so a
    // bare date group wearing garnish prose that
    // happened to contain "to" — "(November 15, note
    // to self)", "(November 15, back to church)" —
    // misclassified as a chain link and suppressed
    // a real garnish mismatch; the test now requires
    // the claim itself (see `group_has_target_claim`).
    // Round 31 (R31-C, F4): a group with no "to" /
    // "until" word in it can carry no phrase-with-to
    // (the test below needs that word standing after
    // its phrase), so its tokens are not built — the
    // chain test used to tokenize every group on the
    // line, thousands of bare date groups included.
    // The screen reads the group's words as the
    // cleaning leaves them: format characters out
    // (a "t\u{200b}o" is the word "to" to the
    // cleaning), case folded — a cleaned "to" or
    // "until" is a substring of that text, so a
    // group the screen passes over holds neither
    // word, exactly as its tokens would say.
    let group_tokens: Vec<Vec<String>> = entries
        .iter()
        .map(|(inner, _)| {
            let screened = strip_format_chars(inner).to_ascii_lowercase();
            if !screened.contains("to") && !screened.contains("until") {
                return Vec::new();
            }
            let raw: Vec<&str> = inner.split_whitespace().collect();
            segment_tokens(&raw)
        })
        .collect();
    let phrase_with_to = |toks: &[String]| {
        (0..toks.len()).any(|ps| {
            // Round 28 (R28-A, F2): the phrase is read
            // in words, as in `target_claim` — its
            // "to" / "until" may stand after its last
            // word, past any nothing in between.
            (1..=2usize).any(|len| {
                let Some((phrase, pe)) = word_phrase(toks, ps, len) else {
                    return false;
                };
                is_target_phrase(&phrase)
                    && toks[pe..]
                        .iter()
                        .any(|w| w.as_str() == "to" || w.as_str() == "until")
            })
        })
    };
    let any_phrase = contributing
        .iter()
        .any(|i| phrase_with_to(&group_tokens[*i]));
    let last_claims = group_has_target_claim(&entries[last].0, read, today, anchor);
    if !any_phrase || !last_claims {
        return Vec::new();
    }
    let mut superseded: Vec<SimpleDate> = Vec::new();
    for i in &contributing[..contributing.len() - 1] {
        if let Some(d) = entries[*i].1 {
            if d != read && !superseded.contains(&d) {
                superseded.push(d);
            }
        }
    }
    superseded
}

/// `text` with every trailing parenthesized group
/// taken off, date groups and garnish alike: the core
/// a plain reading stands on once the groups have
/// been remembered separately.
pub(super) fn strip_all_trailing_parenthesized(text: &str) -> &str {
    // Round 40 (R40-B, F1): the final split's
    // before-part, read off the line's precomputed
    // scan — the same rest the loop here used to
    // walk one strip at a time.
    let scan = LineScan::new(text);
    match scan.trailing_splits().last() {
        Some((before, _)) => before,
        None => text,
    }
}

/// A For Sunday line that names a Sunday in words
/// after a Sunday named in relative words: "the
/// Sunday after next Sunday", "the Sunday after this
/// Sunday" (round 6, RT6-9). Returns the line as
/// written (trimmed, closing punctuation off) when
/// that is the line's whole shape, so the review can
/// repeat the sender's own words; the base must be a
/// relative Sunday — an absolute base is
/// `sunday_after_outcome`'s shape, and anything more
/// is not this shape either.
pub(super) fn sunday_after_relative_phrase(
    raw: &Option<String>,
    today: SimpleDate,
) -> Option<String> {
    let text = raw.as_deref()?.trim();
    // Round 41 (R41-D, F5): the shape and the
    // leading-group gate below are judged on the
    // re-dressed text where the re-dress
    // consultation fires, exactly as in
    // `relative_sunday_phrase` (see there) — a
    // quoted date group standing alone beside a
    // Sunday-after phrase was forgotten with the
    // phrase, and the memory returned is still the
    // line as written.
    let redressed = quoted_groups_redressed_where_judged(text, today, today);
    let judged: &str = redressed.as_deref().unwrap_or(text);
    if !relative_shape_matches(judged, sunday_after_relative_shape_tokens) {
        return None;
    }
    // Round 21 (R21-1), bounded by the R10-10c
    // boundary: a leading group that merely restates
    // the Sunday the phrase itself names stays
    // unread, as a group standing before the phrase
    // always has; a leading group naming a DIFFERENT
    // Sunday is a segment asserting another date, and
    // the phrase is remembered with it, the group
    // named on the parenthesized record.
    // Round 36 (R36-D, F4): the leading group may
    // wear any of the three pairs — the gate stated
    // a literal "(" here, so a bracket or brace
    // leading group never entered this machinery at
    // all: it was adopted as the line's date by the
    // generic reading and the phrase forgotten.
    if matches!(judged.trim_start().chars().next(), Some('(' | '[' | '{')) {
        let resolution = relative_phrase_resolution(judged, today, true);
        if !leading_group_disagrees(judged, today, resolution) {
            return None;
        }
    }
    Some(text.trim_end_matches(['.', ',']).to_string())
}

/// The Sunday a relative phrase in `text` resolves
/// to at parse time (round 21, R21-1): the groups at
/// either end taken off, the remaining shape's
/// "next" / "this" base resolved against `today`,
/// and — for a Sunday-after phrase — the first
/// Sunday after that base.
pub(super) fn relative_phrase_resolution(
    text: &str,
    today: SimpleDate,
    sunday_after: bool,
) -> SimpleDate {
    // Round 40 (R40-B, F1): the strip loop is
    // tracked as a byte range of the one line, its
    // trailing strips read off the line's
    // precomputed scan (`LineScan::split_last`) —
    // the same alternation the loop here used to
    // walk, trailing first, then leading.
    let scan = LineScan::new(text);
    let mut lo = 0usize;
    let mut hi = text.len();
    loop {
        if let Some((before, _, _)) = scan.split_last(lo, hi) {
            hi = before;
            continue;
        }
        match strip_leading_parenthesized(&text[lo..hi]) {
            Some(stripped) => {
                lo = stripped.as_ptr() as usize - text.as_ptr() as usize;
            }
            None => break,
        }
    }
    let rest = &text[lo..hi];
    let next = rest
        .split_whitespace()
        .map(group_token)
        .any(|w| w == "next");
    let base = relative_sunday_date(next, today);
    if sunday_after {
        first_sunday_after(base)
    } else {
        base
    }
}

/// Whether a parenthesized group standing at the
/// very start of `text` contributes a date other
/// than `resolution` (round 21, R21-1): every leading
/// group is read in written order, and the first
/// disagreement answers. A group that contributes
/// nothing, or restates the resolution, does not
/// disagree.
/// Round 36 (R36-D, F4): a leading group may wear
/// any of the three pairs, as everywhere else in
/// the group machinery since round 34 — the loop
/// stated a literal "(" here, so it stopped at (or
/// never entered) a bracket or brace group standing
/// in the leading run, and the groups after it were
/// never judged either. The span walk it calls
/// (`first_balanced_span`) already balances all
/// three pairs, and the span's inner text is read
/// the same way whatever pair stands at its edges.
pub(super) fn leading_group_disagrees(
    text: &str,
    today: SimpleDate,
    resolution: SimpleDate,
) -> bool {
    let mut rest = text.trim_start();
    while matches!(rest.chars().next(), Some('(' | '[' | '{')) {
        // Round 32 (R32-C, F3): the first span only,
        // as in `strip_leading_parenthesized` — the
        // full span list of the remainder was rebuilt
        // for every leading group judged.
        let Some((start, end)) = first_balanced_span(rest) else {
            return false;
        };
        if start != 0 {
            return false;
        }
        let inner = &rest[start + 1..end - 1];
        if let Some(d) = group_date(inner, today, today) {
            if d != resolution {
                return true;
            }
        }
        rest = rest[end..].trim_start();
    }
    false
}

/// Whether `text` is exactly the Sunday-after-a-
/// relative-Sunday shape, trailing weekday garnish
/// and all.
pub(super) fn sunday_after_relative_shape(text: &str) -> bool {
    let words: Vec<String> = text
        .split_whitespace()
        // Round 23 (R23-3): the one shared cleaning
        // (`group_token`), as everywhere else — a
        // separator token standing alone (":" / "/"
        // / a dash where a group was taken off, or
        // wearing a word) cleans to an empty word or
        // a clean word, and is no word of the shape.
        .map(group_token)
        .filter(|w| !w.is_empty())
        .collect();
    sunday_after_relative_shape_tokens(&words)
}

/// The Sunday-after shape judged on the cleaned
/// word list (round 31, R31-C): the same words the
/// text form reads, stated once so the group-
/// stripping loop in `relative_shape_matches` can
/// judge them without re-cleaning the line.
pub(super) fn sunday_after_relative_shape_tokens(words: &[String]) -> bool {
    // Leading garnishes compose in order (round 7,
    // R7-5): an optional "on" and/or "the" in front of
    // the shape changes nothing about the shape.
    let mut core: &[String] = words;
    while matches!(core.first().map(String::as_str), Some("on" | "the")) {
        core = &core[1..];
    }
    // A trailing weekday garnish (round 7, R7-5) is
    // part of the sender's words, remembered with
    // them; without it the same shape must still read.
    let shape = |slice: &[String]| -> bool {
        let base: &[String] = match slice {
            [day, after, rest @ ..] if day.as_str() == "sunday" && after.as_str() == "after" => {
                rest
            }
            _ => return false,
        };
        // The relative base may itself wear a "the".
        let base: &[String] = match base.first().map(String::as_str) {
            Some("the") => &base[1..],
            _ => base,
        };
        match base {
            [which, base_day] => {
                (which.as_str() == "next" || which.as_str() == "this")
                    && base_day.as_str() == "sunday"
            }
            _ => false,
        }
    };
    let names_shape = shape(core)
        || (core.len() > 1
            && weekday_word(core.last().map(String::as_str).unwrap_or("")).is_some()
            && shape(&core[..core.len() - 1]));
    names_shape
}

/// A For Sunday line that names its day only in
/// relative words (round 5, RT5-6): "next Sunday",
/// "this Sunday", "on Sunday", or a bare "Sunday".
/// Returns the line as written (trimmed, closing
/// punctuation off) when that is ALL the line says, so
/// the review can repeat the sender's own words; a
/// line saying anything more is not this shape.
pub(super) fn relative_sunday_phrase(raw: &Option<String>, today: SimpleDate) -> Option<String> {
    let text = raw.as_deref()?.trim();
    // Round 41 (R41-D, F5): the shape and the
    // leading-group gate below are judged on the
    // re-dressed text where the re-dress
    // consultation fires (see
    // `quoted_groups_redressed_where_judged`) — a
    // quoted date group standing alone beside the
    // phrase is a group to the date readers there,
    // and the phrase is its bracket twins' phrase;
    // judged as written, the quoted group is no
    // group, the shape fails, and the phrase was
    // forgotten in silence. The memory returned is
    // still the line as written, quoted dress and
    // all (the R36-D rule): only the judgment
    // moves to the re-dressed text, never the
    // sender's words. The consultation here is
    // judged with `today` as its anchor, as every
    // group judgment in this reader already is.
    let redressed = quoted_groups_redressed_where_judged(text, today, today);
    let judged: &str = redressed.as_deref().unwrap_or(text);
    if !relative_shape_matches(judged, relative_sunday_shape_tokens) {
        return None;
    }
    // Round 21 (R21-1), bounded by the R10-10c
    // boundary, as in `sunday_after_relative_phrase`:
    // a leading group restating the phrase's own
    // Sunday stays unread; a leading group naming a
    // different Sunday is remembered with the phrase.
    // Round 36 (R36-D, F4): the leading group may
    // wear any of the three pairs, as there.
    if matches!(judged.trim_start().chars().next(), Some('(' | '[' | '{')) {
        let resolution = relative_phrase_resolution(judged, today, false);
        if !leading_group_disagrees(judged, today, resolution) {
            return None;
        }
    }
    Some(text.trim_end_matches(['.', ',']).to_string())
}

/// Whether `text` is exactly a bare relative Sunday,
/// trailing weekday garnish and all.
pub(super) fn relative_sunday_shape(text: &str) -> bool {
    let words: Vec<String> = text
        .split_whitespace()
        // Round 23 (R23-3): the one shared cleaning
        // (`group_token`), as in
        // `sunday_after_relative_shape` — a separator
        // token standing alone cleans to an empty word
        // and is no word of the shape.
        .map(group_token)
        .filter(|w| !w.is_empty())
        .collect();
    relative_sunday_shape_tokens(&words)
}

/// The bare relative-Sunday shape judged on the
/// cleaned word list (round 31, R31-C): the same
/// words the text form reads, stated once so the
/// group-stripping loop in `relative_shape_matches`
/// can judge them without re-cleaning the line.
pub(super) fn relative_sunday_shape_tokens(words: &[String]) -> bool {
    // Leading garnishes compose in order (round 7,
    // R7-5): an optional "on" and/or "the".
    let mut core: &[String] = words;
    while matches!(core.first().map(String::as_str), Some("on" | "the")) {
        core = &core[1..];
    }
    let shape = |slice: &[String]| -> bool {
        match slice {
            [day] => day.as_str() == "sunday",
            [which, day] => {
                (which.as_str() == "next" || which.as_str() == "this") && day.as_str() == "sunday"
            }
            _ => false,
        }
    };
    // A trailing weekday garnish (round 7, R7-5) is
    // remembered with the sender's words, as on the
    // Sunday-after shapes; it does not unname the
    // Sunday the words name.
    let names_sunday = shape(core)
        || (core.len() > 1
            && weekday_word(core.last().map(String::as_str).unwrap_or("")).is_some()
            && shape(&core[..core.len() - 1]));
    names_sunday
}

/// Read a date standing at the head of `tokens`,
/// letting the line's remainder follow it (round 8,
/// R8-4): the longest opening run of words that reads
/// as a date, and how many words it took. Words after
/// the date are the remainder, not a parse failure.
pub(super) fn parse_date_prefix(tokens: &[&str], today: SimpleDate) -> Option<(SimpleDate, usize)> {
    // Round 23 (R23-3): a date run may not START on a
    // boundary token — a token the shared cleaning
    // (`group_token`) reduces to nothing is boundary
    // punctuation standing alone (a comma left by a
    // closing paren, a colon, a slash, a dash), never
    // the first word of a date. The flexible reader
    // strips commas globally, so ", November 8" used
    // to read as a date run STARTING on the comma:
    // the boundary token was absorbed into the date's
    // own span, the governor before it stood inside
    // the span's window, and a nested group closing
    // between a governor and its date ("((November 1,
    // cancelled), November 8)") silenced the target
    // its comma twin asserts. A boundary is a boundary
    // wherever it stands; the scan steps over it and
    // starts the run on the date's own first word.
    if tokens.first().is_some_and(|t| group_token(t).is_empty()) {
        return None;
    }
    // Round 22 (R22-2): a sender's semicolon (or colon)
    // attached to a date token — "2026;" — is a clause
    // break standing after the date, never part of the
    // date itself: the token is read without it, exactly
    // as `group_token` cleans it for the governor scans.
    // Without this, the year fell outside the date's own
    // span, the stray year token broke the governor
    // scope behind it, and a cancelled date restated
    // after its semicolon resurrected — a sender's
    // semicolon must be indistinguishable from the
    // composition's internal group marker, which the
    // date readers never absorb into a date run.
    // Round 27 (R27-A, F3): a token that is nothing
    // but format characters standing INSIDE a
    // date's own span ("November \u{200b} 1", "to
    // November \u{200b} 8") is nothing — the date
    // is read across it, and the span it reports
    // (`used`) still covers it, so every reader
    // that steps by the span steps over the token
    // too. A Cf-only token at the span's head keeps
    // the boundary rule above (the caller's scan
    // steps over it and starts the run on the
    // date's own first word, as for any boundary
    // token), and a token of real garnish ("*", a
    // standalone ";") is never dropped here: only
    // `cf_only_token` tokens are.
    // Round 28 (R28-A, F2 = review MINOR-3): the
    // drop judges the NOTHING class (see
    // `nothing_token`) — a separator-only token
    // inside the span ("November * 1") is nothing
    // in the date exactly as a Cf-only token is,
    // and reads across the same way. A standalone
    // ";" or bullet inside a span is not a nothing
    // and is still never dropped.
    // Round 30 (R30-D, F7): the reader's word budget
    // bounds both the build and the search below.
    // `parse_flexible_date` reads at most four words —
    // a weekday, a month, a day, and a year (or one
    // numeric token) — and every word past the slots
    // makes the read fail, so a prefix holding five
    // reader words can never be the longest-first
    // hit, and no token past it can stand in a
    // reading either. Words are counted the way the
    // reader counts them (commas are spaces to it;
    // a nothing word is nothing to it), over the
    // normalized tokens, so the bound is exact:
    // every prefix the old search could read is
    // still tried, and every skipped prefix is one
    // it read as None. Before this bound, each call
    // rebuilt the whole remaining line and re-parsed
    // every prefix of it, longest first, so a line
    // of repeated dates cost cubic time in its
    // length (round 30, RT30-4: ≈8–9 s at 3.4 KB,
    // over 120 s at 34 KB).
    let mut kept: Vec<usize> = Vec::new();
    let mut normalized: Vec<String> = Vec::new();
    let mut reader_words = 0usize;
    let mut search_len = 0usize;
    for (idx, &t) in tokens.iter().enumerate() {
        if idx != 0 && nothing_token(t) {
            continue;
        }
        let norm = {
            // A token of nothing but the punctuation
            // (a standalone ";" — the composition's own
            // group marker) keeps its shape: it is a
            // segment boundary no date run may start
            // on or cross, never an empty word.
            // Round 23 (R23-3): the same boundary
            // punctuation ATTACHED at a token's head —
            // a separator written with no space after
            // a group, "(please):November 1, 2026,
            // cancelled", or the composition's own ";"
            // marker glued to the core's first word —
            // is boundary, not part of the word: the
            // token is read without it, exactly as at
            // its tail.
            // Round 24 (R24-4): a bracket at a date
            // token's edge is boundary of the same
            // kind in a date span — "[November 8,
            // 2026]" reads its date on the date's own
            // merits (governance, framing, the fence)
            // instead of the bracket poisoning the
            // parse. Brackets are still not group
            // delimiters: a lone "[November 8]" gains
            // no group reading and stays unread, the
            // recorded limitation.
            // Round 25 (R25-A, M6): format
            // characters are stripped from the date
            // token as at every cleaning site, so
            // one glued inside a month name cannot
            // poison the parse.
            // Round 25 (R25-B, M2 = red team RT25-6):
            // the edge trim is by character CLASS,
            // not a list: any character that is not
            // a letter or a digit — the same garnish
            // class the token rule
            // (`token_has_no_letters_or_digits`)
            // treats as boundary — is trimmed at a
            // token's head and tail, so sender garnish
            // glued to a date ("*November 1", "🎉
            // November 1", "•November 1", "2026*")
            // cannot poison the parse either, and a
            // governed first date wearing it stays
            // governed exactly as its clean twin is.
            // A token of nothing but garnish trims to
            // empty and keeps its shape below: it is
            // a boundary, never an empty word. The
            // class stops at the two parentheses:
            // they are the system's structural group
            // delimiters, not garnish — a date token
            // wearing one stands at a group edge, and
            // the raw-text scans rely on this parse
            // failing there (the leading-group and
            // relative-memory compositions, R10-B /
            // R21-A / R22-A / R23-A / R24-A), exactly
            // as the class already stopped at nothing
            // else the old list named.
            let stripped = strip_format_chars(t);
            let trimmed = stripped
                .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '(' && c != ')')
                .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '(' && c != ')');
            if trimmed.is_empty() {
                t.to_string()
            } else {
                trimmed.to_string()
            }
        };
        reader_words += reader_word_count(&norm);
        kept.push(idx);
        normalized.push(norm);
        if reader_words <= 4 {
            search_len = kept.len();
        } else {
            break;
        }
    }
    (1..=search_len).rev().find_map(|end| {
        parse_flexible_date(&normalized[..end].join(" "), today).map(|d| (d, kept[end - 1] + 1))
    })
}

/// The number of words one normalized token holds
/// for `parse_flexible_date` (round 30, R30-D): the
/// reader turns commas into spaces before it splits,
/// and a word that is nothing (see `nothing_token`)
/// is no word to it. Counted on the normalized token
/// — the very string the reader is handed — so the
/// budget in `parse_date_prefix` counts exactly the
/// words a prefix of the line presents to the reader.
fn reader_word_count(token: &str) -> usize {
    token
        .replace(',', " ")
        .split_whitespace()
        .filter(|piece| !nothing_token(piece))
        .count()
}

/// A token with its wrapping brackets and sentence
/// punctuation off: the cleaning the framing readers
/// share, stated once (health audit E-8 — it used to
/// be defined verbatim inside both `weekday_is_framed`
/// and `framed_date_outcome`).
pub(super) fn clean(t: &str) -> &str {
    // Round 34 (R34-C, F4 = RT34-5): the trim set
    // gains the remaining sentence marks — ";", ":",
    // "!", "?", and the closing quote — so a weekday
    // word wearing one is still the word it names,
    // as `weekday_word` already honors ";" and ":"
    // as boundary endings.
    // Round 35 (R35-A, F1 = review R35-R1): the set
    // gains the brace pair — a weekday word wearing
    // braces ("{Monday}") is the word it names, as
    // its paren and bracket twins already are here.
    // Round 35 (R35-C, F7 = red team RT35-5): the set
    // gains the apostrophe and the typographic
    // closing quote — a weekday word wearing either
    // ("(Monday)'", "(Monday)”") is the word it
    // names, as one wearing ";" or the closing quote
    // already is.
    // Round 37 (R37-E, F3): the set gains the rest
    // of the curly family — the opening quotes and
    // the single closing quote join the double
    // closing quote — so a weekday word wearing a
    // curly pair ("“Monday” November 1, 2026", the
    // trailing "November 1, 2026 “Monday”") is the
    // word it names, as one wearing the straight
    // pair already is.
    // Round 40 (R40-G, F8): the set gains the
    // ellipsis U+2026 — a weekday word wearing one
    // ("Monday…", "(Monday)…") is the word it
    // names, as one wearing ";" or "!" already is;
    // the suffix stood in no reader's set, so it
    // disarmed the framing judgment it followed
    // and the bare weekday lost its record.
    t.trim_matches([
        '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
        '\u{201c}', '\u{2018}', '\u{2019}', '\u{2026}',
    ])
}

/// Whether the weekday at `i` in `core` wears the
/// framing garnish stack (round 8, R8-6): an optional
/// "on" and an optional "the" in front of it — the
/// stack the Sunday-after reader strips — or stands
/// at the line's start bare, as the leading form
/// always has. A weekday wearing neither, away from
/// the line's start, frames nothing and stays
/// ordinary text (round 5, R5-D).
pub(super) fn weekday_is_framed(core: &[&str], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    // Round 27 (R27-A, F3): the framing words are
    // the nearest words that are something before
    // the weekday — a Cf-only token ("on \u{200b}
    // Sunday") stands between them and the weekday
    // as nothing.
    let mut prev: Vec<usize> = Vec::new();
    let mut j = i;
    while j > 0 && prev.len() < 2 {
        j -= 1;
        if !cf_only_token(core[j]) {
            prev.push(j);
        }
    }
    let Some(&p1) = prev.first() else {
        return false;
    };
    if clean(core[p1]).eq_ignore_ascii_case("on") {
        return true;
    }
    if clean(core[p1]).eq_ignore_ascii_case("the") {
        // "on the Monday", or the line's own opening
        // "the" standing in front of the weekday
        // (no word that is something before it).
        return match prev.get(1) {
            None => true,
            Some(&p2) => clean(core[p2]).eq_ignore_ascii_case("on"),
        };
    }
    false
}

/// A weekday wearing "on" directly in front of a
/// date frames that date wherever it stands in the
/// line (round 7, R7-3), and a leading framing weekday
/// and a trailing one on the same date are each
/// checked (round 7, R7-7). Returns the read date, or
/// the first disagreeing framing's mismatch, when a
/// framing weekday stands in one of those places and
/// the date it frames reads; a weekday with no "on"
/// away from the line's start frames nothing here and
/// stays ordinary text (round 5, R5-D).
///
/// Round 8 (R8-4/R8-5/R8-6): the framing needs the
/// weekday directly in front of the date — what
/// follows the date is the line's remainder, not a
/// parse failure — the reader scans EVERY framing
/// candidate in the line until one frames a date (a
/// candidate whose following words do not parse as a
/// date is not a framing, and scanning continues),
/// and the weekday may wear the garnish stack, an
/// optional "on" and an optional "the". The trailing
/// weekday frames the same date only when it stands
/// directly after it; words between the date and a
/// trailing weekday word make that word part of the
/// remainder, not a second framing.
pub(super) fn framed_date_outcome(
    text: &str,
    today: SimpleDate,
) -> Option<(Option<SimpleDate>, Option<WeekdayMismatch>)> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let last = tokens
        .iter()
        .rposition(|t| t.chars().any(|c| c.is_alphanumeric()))?;
    let trailing_word = clean(tokens[last]);
    // Round 39 (R39-E, F7): the framing readers'
    // cleaning sheds the interior sentence marks
    // from a weekday word (see `cleaned_weekday`,
    // whose scope round 40 (R40-F) extended to the
    // bare path's whole scope — runs and a second
    // mark), as it does at the garnish-group reader
    // — a grouped "(Mon!day)" frames its date here
    // exactly as the bare "Mon!day" does.
    let trailing = cleaned_weekday(trailing_word);
    let core: Vec<&str> = match trailing {
        Some(_) => tokens
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != last)
            .map(|(_, t)| *t)
            .collect(),
        None => tokens.clone(),
    };
    for i in 0..core.len() {
        let Some((lead_named, lead_number)) = cleaned_weekday(clean(core[i])) else {
            continue;
        };
        if !weekday_is_framed(&core, i) {
            continue;
        }
        let Some((date, used)) = parse_date_prefix(&core[i + 1..], today) else {
            continue;
        };
        if date.weekday() != lead_number {
            let named = lead_named;
            return Some((None, Some(WeekdayMismatch { named, date })));
        }
        if i + 1 + used == core.len() {
            if let Some((trailing_word, trailing_number)) = trailing {
                if date.weekday() != trailing_number {
                    let named = trailing_word;
                    return Some((None, Some(WeekdayMismatch { named, date })));
                }
            }
        }
        return Some((Some(date), None));
    }
    None
}

/// The weekday a garnish-sequence group names when
/// it holds one weekday word and nothing else
/// (round 37, R37-D, F7): the group's inner text is
/// one token, read through the shared cleaning
/// (`clean`) exactly as the lone garnish token is
/// read at the framing and trailing sites — a
/// wrapper the group wears inside its own pair
/// ("[Monday]" standing inside "([Monday])") cleans
/// off with the pair's marks — and the word is
/// returned as the lone record names it. Round 39
/// (R39-E, F7): the word is read through
/// `cleaned_weekday`, so its interior sentence
/// marks are shed under the weekday vocabulary
/// anchor (the bare path's whole scope since
/// round 40, R40-F) — "(Mon!day)" names Monday
/// here as its bare twin does.
pub(super) fn garnish_group_weekday(inner: &str) -> Option<(String, u32)> {
    let mut words = inner.split_whitespace();
    let word = words.next()?;
    if words.next().is_some() {
        return None;
    }
    cleaned_weekday(clean(word))
}

/// The word with every interior sentence mark of
/// the shed set ("!", "?", ":", ";") dropped — the
/// ONE shed both interior arms state (round 40,
/// R40-F, F7): `bare_date_mark_shed`'s interior arm
/// joins a letter run's letters through this shed,
/// and `cleaned_weekday` sheds a grouped word
/// through it, so the two paths hold the same mark
/// set and shed the same marks by construction and
/// cannot drift apart on scope again. An interior
/// "." is not in the set, as it is in neither arm:
/// "." is a sentence separator in this program, not
/// a mark of the shed family (see the interior-"."
/// stop recorded in
/// `r40f_interior_period_family_keeps_its_standing_readings`).
pub(super) fn interior_mark_shed(word: &str) -> String {
    word.chars()
        .filter(|c| !matches!(c, '!' | '?' | ':' | ';'))
        .collect()
}

/// A weekday word as the grouped readers name it:
/// the word itself when it is one, or the word with
/// its interior ASCII sentence marks shed when that
/// shed completes a weekday name (round 39, R39-E,
/// F7 = review R39-R7). The grouped readers trim a
/// token's edges but not its interior, so "(Mon!day)"
/// was inert garnish at every garnish slot while the
/// bare twin "Mon!day" refused with the record —
/// `bare_date_mark_shed`'s interior arm sheds the
/// mark on the bare path, guarded to marks OUTSIDE
/// every group, and this arm is its grouped twin:
/// the same mark set ("!", "?", ":", ";"), the same
/// vocabulary anchor (`weekday_word` on the shed
/// word, so the shed completes only a weekday name
/// and no other word — a governor's stem above all —
/// is ever completed by shedding), and the shed
/// word is the name the record carries, as the bare
/// twin's record names the shed word. A mark at the
/// word's edge is the edge trims' business, not this
/// arm's.
///
/// Round 40 (R40-F, F7 = red team RT40-F + review
/// R40-R4): the shed's scope is the bare path's
/// standing scope whole, no longer one mark. R39-F
/// extended the bare interior arm to a mark RUN and
/// a second interior mark — its letter run sheds
/// every mark it holds when the letters joined are
/// a month or weekday name ("Mon!!day", "Mo!n!day",
/// "Mon!da!y", and past two, "M!o!n!day" and
/// "Mon!!!day" all refuse bare) — and the grouped
/// arm never took the extension, so the two-mark
/// grouped forms served silently at all three
/// garnish slots. The grouped shed now takes the
/// same scope by the same statement: a word holding
/// only letters and marks of the set, a letter at
/// each edge (an edge mark is the trims' business),
/// and at least one mark, sheds EVERY mark it holds
/// through `interior_mark_shed`, and the shed word
/// names the weekday exactly when `weekday_word`
/// names it — the vocabulary anchor that keeps the
/// governor-completion boundary (no governor word
/// is completed by this shed, at any mark count).
pub(super) fn cleaned_weekday(word: &str) -> Option<(String, u32)> {
    if let Some(number) = weekday_word(word) {
        return Some((word.to_string(), number));
    }
    let bytes = word.as_bytes();
    if !bytes.first().is_some_and(|b| b.is_ascii_alphabetic())
        || !bytes.last().is_some_and(|b| b.is_ascii_alphabetic())
    {
        return None;
    }
    if !bytes
        .iter()
        .all(|b| b.is_ascii_alphabetic() || matches!(b, b'!' | b'?' | b':' | b';'))
    {
        return None;
    }
    if !bytes.iter().any(|b| matches!(b, b'!' | b'?' | b':' | b';')) {
        return None;
    }
    let shed = interior_mark_shed(word);
    weekday_word(&shed).map(|number| (shed, number))
}

/// `rest` (already trimmed at its head) with the
/// framing garnish stack taken off: an optional
/// "on" and an optional "the", in the compositions
/// the framing readers allow — the stack that may
/// stand in front of a lone weekday group at the
/// leading slot ("on (Monday) ...", "the (Monday)
/// ...", "on the (Monday) ..."), read here so a
/// sequence standing behind the same stack is
/// reached the same way.
fn skip_framing_garnish(mut rest: &str) -> &str {
    for _ in 0..2 {
        let Some(token) = rest.split_whitespace().next() else {
            break;
        };
        let cleaned = clean(token);
        if !cleaned.eq_ignore_ascii_case("on") && !cleaned.eq_ignore_ascii_case("the") {
            break;
        }
        rest = rest[token.len()..].trim_start();
    }
    rest
}

/// `text` with a leading governor group unwrapped:
/// the group's own governor words standing bare at
/// the leading slot, every other byte as written
/// (round 38, R38-D, F4 — a doctrine change adopted
/// openly in the round-38 synthesis, adjudication
/// 4, not a reading smuggled into this batch).
///
/// The doctrine: a leading group's governor
/// contents are judged as the bare governor is
/// judged, at the leading slot, in every wrapper.
/// The slot already judges group contents — for
/// weekdays (the framing readers and the R37-D
/// sequence walk) and for dates (the R36-D / R37-B
/// consultation beside relative phrases) — and on
/// the audited tree the governor was the one exempt
/// contents class: a wrapped leading governor was
/// garnish in every wrapper form (all three pairs,
/// doubled, mixed, marked, glued, spaced, behind
/// the `on`/`the` framing stack, with a companion,
/// before a Sunday-after phrase) while the bare
/// governor governed in every corresponding form
/// and the quote-wrapped bare governor governed
/// too — inside one leading sequence the weekday
/// judged and the cancellation standing beside it
/// was ignored. Dress is not semantics at a judged
/// slot.
///
/// The construction is the judgment restated as a
/// reading: the framing garnish stack aside, every
/// top-level group standing in the leading run
/// (separated as the sequence walk allows) whose
/// whole contents are a governor phrase — judged
/// by the standing phrase judgments
/// (`is_governor_phrase` / `is_cancellation_phrase`)
/// over the group's cleaned words, the same
/// judgments the bare line's segment scan applies —
/// is replaced by those words bare, so every reader
/// below reads the line its bare-governor twin
/// reads, field for field, on both routes.
/// Companion groups keep their dress and their own
/// standing judgments: only the governor's wrapper
/// comes off. A group holding a date, a weekday, or
/// inert words alone is not this pass's: its
/// standing readers judge it whole. Round 39
/// (R39-H2): a group whose contents mix a governor
/// phrase with other words IS this pass's — the
/// contents are judged as the bare governor text is
/// judged (see `governor_group_words`) — at this
/// slot as at the trailing and mid-line slots.
///
/// Scope, per the coordinator's ruling
/// (r38d-ruling.md, option (a)): the LEADING slot
/// only. The walk starts at the line's head and
/// stops at the first word that is not a run group,
/// so a group standing anywhere else — trailing,
/// at a Sunday-after base's trailing slot, mid-line
/// — keeps its standing reading byte for byte this
/// round; the wrapped/bare split at those slots is
/// carried to round 39 as a doctrine question, not
/// widened here. Round 39 (R39-H, F10) adjudicated
/// that question and extended the doctrine to the
/// trailing slot and the date's segment mid-line;
/// that extension is the companion pass
/// (`unwrapped_trailing_or_midline_governor_group`)
/// below, and this pass's scope is unchanged.
fn unwrapped_leading_governor_group(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let head_offset = text.len() - trimmed.len();
    let rest = skip_framing_garnish(trimmed);
    let mut scan = head_offset + (trimmed.len() - rest.len());
    let mut out: Option<String> = None;
    let mut cursor = 0usize;
    loop {
        // The separations the leading sequence walk
        // allows between the run's groups (round 38,
        // R38-B): whitespace, commas, the sentence
        // separators, and a typographic closing quote
        // standing as a group's suffix. Round 39
        // (R39-C, F4, arm (b)): the skip set gains the
        // three suffix marks the lone forms judge a
        // group wearing — "!", "?", and U+201D — so a
        // companion wearing one ("(notes)” (cancelled)
        // November 8, 2026") no longer shields the
        // governor group standing after it.
        let mut p = scan;
        while let Some(c) = text[p..].chars().next() {
            if matches!(
                c,
                ' ' | '\t' | ',' | ';' | '.' | ':' | '!' | '?' | '\u{201d}' | '\u{2019}'
            ) {
                p += c.len_utf8();
            } else {
                break;
            }
        }
        if !matches!(text[p..].chars().next(), Some('(' | '[' | '{')) {
            break;
        }
        let Some((start, end)) = first_balanced_span(&text[p..]) else {
            break;
        };
        if start != 0 {
            break;
        }
        let inner = &text[p + 1..p + end - 1];
        if let Some(words) = governor_group_words(inner) {
            let bare = words.join(" ");
            let buf = out.get_or_insert_with(|| String::with_capacity(text.len()));
            buf.push_str(&text[cursor..p]);
            // The bare words stand as words: a space
            // separates them from a word or a group
            // they stand glued against, on either
            // side ("(Monday)(cancelled)November" —
            // the governor's twin is "(Monday)
            // cancelled November").
            if buf
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || matches!(c, ')' | ']' | '}'))
            {
                buf.push(' ');
            }
            buf.push_str(&bare);
            let after = p + end;
            if text[after..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || matches!(c, '(' | '[' | '{'))
            {
                buf.push(' ');
            }
            cursor = after;
        }
        scan = p + end;
    }
    let mut buf = out?;
    buf.push_str(&text[cursor..]);
    Some(buf)
}

/// The words of a group whose contents carry a
/// governor, when they do (round 38, R38-D; stated
/// once for the governor passes): the group's cleaned
/// words — `segment_tokens` over its inner words, the
/// standing cleaning — judged as the bare governor
/// text is judged. The pure arm is the standing one:
/// the whole contents are a governor phrase, judged
/// by the standing phrase judgments
/// (`is_governor_phrase` / `is_cancellation_phrase`).
/// Round 39 (R39-H2, the impure single-group family)
/// adds the impure arm: the bare line's segment scan
/// does not judge a run of words whole — it finds a
/// governor phrase at every phrase position in the
/// run (the scan in `segment_tokens`) — so contents
/// mixing a governor phrase with other words
/// ("cancelled today", "today cancelled", "meeting
/// cancelled") are judged by that same scan: a
/// governor phrase standing at any position of the
/// cleaned words makes the group a governor group,
/// and the unwrapping stands ALL its words bare, so
/// the line reads its bare-contents twin word for
/// word. Two holdings stay outside both arms, as
/// they are not governor text in any dress: contents
/// holding a date (a month word, a weekday word, or
/// a digit stands among the words — a date-carrying
/// or weekday-carrying group is judged whole by its
/// standing readers, its split a different family's),
/// and contents holding no governor phrase at any
/// position (inert words alone) yield None: their
/// standing readers judge them whole, at every slot.
fn governor_group_words(inner: &str) -> Option<Vec<String>> {
    let raw_words: Vec<&str> = inner.split_whitespace().collect();
    let words: Vec<String> = segment_tokens(&raw_words)
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return None;
    }
    if is_governor_phrase(&words) || is_cancellation_phrase(&words) {
        return Some(words);
    }
    // The impure arm (R39-H2): a governor phrase at
    // any phrase position of the contents, judged by
    // the segment scan's own window (one to three
    // words at every start), in contents holding no
    // date and no weekday.
    let holds_date_or_weekday = words.iter().any(|w| {
        word_at_is_month(w, 0) || weekday_word(w).is_some() || w.chars().any(|c| c.is_ascii_digit())
    });
    if holds_date_or_weekday {
        return None;
    }
    let has_governor_position = (0..words.len()).any(|s| {
        (1..=3usize).any(|len| {
            words
                .get(s..s + len)
                .is_some_and(|p| is_governor_phrase(p) || is_cancellation_phrase(p))
        })
    });
    if has_governor_position {
        Some(words)
    } else {
        None
    }
}

/// `text` with the given group spans (byte ranges,
/// in written order) replaced by their governor
/// words standing bare, every other byte as written
/// — the replacement the governor passes share: a
/// space separates the bare words from a word or a
/// group they stand glued against, on either side,
/// as in `unwrapped_leading_governor_group`.
fn apply_governor_unwraps(text: &str, unwraps: &[(usize, usize, Vec<String>)]) -> Option<String> {
    if unwraps.is_empty() {
        return None;
    }
    let mut buf = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for (start, end, words) in unwraps {
        buf.push_str(&text[cursor..*start]);
        if buf
            .chars()
            .last()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, ')' | ']' | '}'))
        {
            buf.push(' ');
        }
        buf.push_str(&words.join(" "));
        if text[*end..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '(' | '[' | '{'))
        {
            buf.push(' ');
        }
        cursor = *end;
    }
    buf.push_str(&text[cursor..]);
    Some(buf)
}

/// The last plain date a token scan of `text`
/// reads, when one stands: every token position is
/// tried as a date's start (`parse_date_prefix`,
/// whose word budget bounds each try), and the last
/// date read in written order is the text's nearest
/// preceding date for a group standing after it.
fn last_plain_date_in(text: &str, today: SimpleDate) -> Option<SimpleDate> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut last = None;
    for i in 0..tokens.len() {
        if let Some((date, _)) = parse_date_prefix(&tokens[i..], today) {
            last = Some(date);
        }
    }
    last
}

/// Whether the nearest plain date standing in
/// `prefix` (the text before a group) is a
/// Sunday-after phrase's consumed base: the group
/// then stands in base-trailing company, where the
/// governor is garnish in every dress (round-39
/// synthesis, adjudication 4 — the base-trailing
/// slot is uniform and does not move), and no
/// governor pass unwraps it there.
fn nearest_date_is_sunday_after_base(prefix: &str, today: SimpleDate) -> bool {
    match last_plain_date_in(prefix, today) {
        Some(date) => sunday_after_consumed_bases(prefix, today).contains(&date),
        None => false,
    }
}

/// The group run closing `text` (one or more
/// top-level groups standing at the line's end,
/// sentence punctuation and a trailing separator
/// token aside, as the standing trailing readers
/// peel them), when one stands: the run's spans in
/// written order and the byte where the run starts.
/// The run's members stand against each other with
/// only the sequence walks' separations between
/// them — whitespace, commas, the sentence
/// separators ";" "." ":", and a group's suffix
/// marks "!" "?" U+201D U+2019 (round 39, R39-C) —
/// a glued pair's gap empty: the trailing walk's
/// own run definition (`trailing_group_sequence`),
/// stated for a run of any length, a single group
/// included, because the lone trailing group is
/// this slot's headline form.
fn trailing_group_run(text: &str) -> Option<(Vec<(usize, usize)>, usize)> {
    let mut end = text.len();
    loop {
        // Round 40 (R40-G, F8): the peel set gains
        // the ellipsis U+2026 — a run closing the
        // line in front of one ("November 8, 2026
        // (cancelled)…") is the run it names, so
        // the trailing governor unwraps and governs
        // instead of the cancelled date serving.
        // Round 41 (R41-E, F6): the peel set gains
        // the rest of the R40-G sequence-tail set —
        // "~", "*", "#", "-", "—" (U+2014) — which
        // R40-G judged only at the leading slot, so
        // a run closing the line in front of one
        // ("November 8, 2026 (cancelled)~") was no
        // run at all and its cancelled date served
        // live; the spaced form ("(cancelled) ~")
        // is a suffix token of the same set, taken
        // with the standing separator tokens below.
        let peeled = text[..end]
            .trim_end_matches([
                '.', ',', ';', ':', '!', '?', '"', '\'', '\u{201d}', '\u{2019}', '\u{2026}', '~',
                '*', '#', '-', '\u{2014}',
            ])
            .len();
        if peeled != end {
            end = peeled;
            continue;
        }
        let core = &text[..end];
        let Some((ws, wc)) = core.char_indices().rev().find(|(_, c)| c.is_whitespace()) else {
            break;
        };
        let token = &core[ws + wc.len_utf8()..];
        if !token.is_empty()
            && token
                .chars()
                .all(|c| matches!(c, '*' | '|' | '~' | '#' | '-' | '\u{2014}'))
        {
            end = ws;
            continue;
        }
        break;
    }
    let core = text[..end].trim_end();
    let spans = balanced_spans(core);
    let last = spans.len().checked_sub(1)?;
    if spans[last].1 != core.len() {
        return None;
    }
    let mut run_first = last;
    while run_first > 0 {
        let gap = &core[spans[run_first - 1].1..spans[run_first].0];
        if !gap.chars().all(|c| {
            matches!(
                c,
                ',' | ';' | '.' | ':' | '!' | '?' | '\u{201d}' | '\u{2019}'
            ) || c.is_whitespace()
        }) {
            break;
        }
        run_first -= 1;
    }
    Some((spans[run_first..=last].to_vec(), spans[run_first].0))
}

/// Whether the group at `spans[idx]` stands in the
/// date's segment before a plain date (see
/// `unwrapped_trailing_or_midline_governor_group`):
/// companion groups and the sequence walks'
/// separations aside, a plain date stands directly
/// after the run the group belongs to, and the
/// group does not stand after a "Sunday ... after"
/// head the base has not followed — nor in
/// base-trailing company (its nearest preceding
/// plain date the phrase's consumed base). Round
/// 40 (R40-D, F5): a group standing after the
/// phrase's own "after", with the base standing
/// after the group's run, is the Sunday-after
/// interior slot, judged since that round (see
/// the interior test in the body); the head guard
/// above it is the split position's and the
/// before-phrase shapes', where no plain date
/// follows the group.
fn midline_governor_before_plain_date(
    text: &str,
    spans: &[(usize, usize)],
    idx: usize,
    today: SimpleDate,
) -> bool {
    let (start, mut pos) = spans[idx];
    let prefix = &text[..start];
    let raw_words: Vec<&str> = prefix.split_whitespace().collect();
    let words: Vec<String> = segment_tokens(&raw_words)
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect();
    // Round 40 (R40-D, F5): the Sunday-after
    // interior slot is no longer held by the
    // open-phrase guard below. Round 40 graded the
    // carried neighbour (synthesis adjudication 4):
    // governor contents standing between the
    // phrase's "after" and its base are judged as a
    // governor in every dress, so a governor group
    // there unwraps as at every other mid-line slot
    // this pass judges — subject to the same checks
    // below (the base-trailing test, and a plain
    // date standing after the group's run: a
    // weekday-bearing base fails that test as it
    // does at every mid-line slot, so the
    // weekday-bearing family at this slot keeps its
    // standing refusal). The slot is read off the
    // prefix's PLAIN words (the text between its
    // spans, as `group_split_sunday_after_phrase`
    // tokenizes them), so a companion group
    // standing in the run before this group
    // ("Sunday after (Monday)(cancelled) ...")
    // contributes no word to the prefix: the
    // phrase's "Sunday" stands immediately before
    // its "after", and nothing but the framing
    // stack ("the" / "on") stands between that
    // "after" and this group's run.
    let mut plain_words: Vec<String> = Vec::new();
    let mut seg_start = 0usize;
    for &(s, e) in &spans[..idx] {
        plain_words.extend(
            text[seg_start..s]
                .split_whitespace()
                .map(group_token)
                .filter(|w| !w.is_empty()),
        );
        seg_start = e;
    }
    plain_words.extend(
        text[seg_start..start]
            .split_whitespace()
            .map(group_token)
            .filter(|w| !w.is_empty()),
    );
    let interior = plain_words
        .iter()
        .rposition(|w| w == "after")
        .is_some_and(|p| {
            p > 0
                && plain_words[p - 1] == "sunday"
                && plain_words[p + 1..].iter().all(|w| w == "the" || w == "on")
        });
    if !interior {
        if words.last().is_some_and(|w| w == "after") {
            return false;
        }
        if let Some(after_pos) = words.iter().rposition(|w| w == "after") {
            let sunday_before = words[..after_pos].iter().any(|w| w == "sunday");
            let base_after = words[after_pos + 1..]
                .iter()
                .any(|w| word_at_is_month(w, 0));
            if sunday_before && !base_after {
                return false;
            }
        }
    }
    if nearest_date_is_sunday_after_base(prefix, today) {
        return false;
    }
    for &(next_start, next_end) in &spans[idx + 1..] {
        let gap = &text[pos..next_start];
        if !gap.chars().all(|c| {
            matches!(
                c,
                ',' | ';' | '.' | ':' | '!' | '?' | '\u{201d}' | '\u{2019}'
            ) || c.is_whitespace()
        }) {
            break;
        }
        pos = next_end;
    }
    let rest = text[pos..].trim_start_matches([
        ' ', '\t', ',', ';', '.', ':', '!', '?', '\u{201d}', '\u{2019}',
    ]);
    if matches!(rest.chars().next(), Some('(' | '[' | '{')) {
        return false;
    }
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    // Round 40 (R40-D, F5): at the interior slot
    // the base may wear its framing stack ("the" /
    // "on" standing between the group and the base
    // is the base's own dress, as the split-phrase
    // guard's base reading allows it), so the plain
    // date is read past the stack there. At every
    // other slot the standing test is untouched.
    let skip = if interior {
        tokens
            .iter()
            .take_while(|t| matches!(group_token(t).as_str(), "the" | "on"))
            .count()
    } else {
        0
    };
    parse_date_prefix(&tokens[skip..], today).is_some()
}

/// `text` with a trailing-slot or mid-line governor
/// group unwrapped: the group's own governor words
/// standing bare at the slot, every other byte as
/// written (round 39, R39-H, F10 — the doctrine
/// change adjudicated in the round-39 synthesis,
/// adjudication 3, extending the round-38 leading
/// doctrine of `unwrapped_leading_governor_group`).
///
/// The doctrine statement, as adjudicated: governor
/// contents in a group are judged as the bare
/// governor is judged wherever the bare governor is
/// judged — the leading slot (R38-D), the trailing
/// slot, and the date's segment mid-line; weekday-
/// group judgment remains framing-slot only (cell
/// (a)(i) stands closed, not re-opened — a weekday
/// companion in a trailing or mid-line run keeps
/// its own standing judgment, and only the
/// governor's wrapper comes off, so the companions'
/// judgments stand as the walks give them).
///
/// The trailing slot is the group run closing the
/// line after a plain date (see
/// `trailing_group_run`): every wrapper, glued,
/// doubled, mixed, marked, with companions in
/// either order. Inside a judged trailing sequence
/// the dress decided outcomes before this pass:
/// beside a date group the wrapped form served the
/// written date with the group as garnish
/// ("November 1, 2026 (cancelled)(November 8, 2026)"
/// served November 1) where the bare form served
/// the group's date with the written date demoted;
/// beside a disagreeing weekday group the wrapped
/// form left the sender-correcting record standing
/// where the bare form deleted it with the date.
/// The unwrapped line is the bare twin, so every
/// reader below reads the line its bare-governor
/// twin reads, field for field, on both routes.
///
/// The mid-line form is a governor group standing
/// in the date's segment before a plain date: the
/// bare stamp's judgment there is a segment
/// judgment, not a slot judgment — it governs
/// anywhere inside the date's segment — and the
/// wrapped form's whole exemption was one ordinary
/// word standing before it ("notes (cancelled)
/// November 8, 2026" served in every dress while
/// "notes cancelled November 8, 2026" governs).
/// This arm is scoped to where the bare stamp
/// itself governs: before a plain date in-segment.
///
/// Where the bare stamp is itself garnish, the
/// wrapped form does not move, by construction:
/// after a Sunday-after base in every dress (the
/// base-trailing slot is uniform — a run whose
/// nearest preceding plain date is the phrase's
/// consumed base is never unwrapped, and a mid-line
/// group in that company is held by the same test);
/// before phrases in both dresses (no plain date
/// stands after the group — a Sunday-after phrase,
/// a relative phrase — so neither arm reaches it);
/// beside a weekday group mid-line in the order
/// where the bare stamp lapses (the governor
/// standing before the weekday group unwraps to
/// that very lapsing twin — "notes cancelled
/// (Monday) November 1, 2026" — and the line serves
/// as it served); at the Sunday-after phrase's
/// split position, where no plain date follows the
/// group; and a group followed by more words at
/// the line's end, which is not the trailing slot
/// (the run must close the line, as the walks
/// define it). A group holding a date, a weekday,
/// or inert words alone is not this pass's either
/// (see `governor_group_words`), at any slot,
/// exactly as at the leading slot since R38-D; a
/// group whose contents mix a governor phrase with
/// other words is this pass's since R39-H2, at the
/// slots this pass already judges and no others —
/// the slot guards above (the base-trailing
/// exclusion, the open-phrase guard outside the
/// interior slot, the trailing run closing the
/// line) are untouched, so the impure dress moves
/// only where the pure dress already moved. Round
/// 40 (R40-D, F5, adjudication 4): the Sunday-after
/// interior slot — a governor group standing
/// between the phrase's "after" and its base, in
/// every dress (spaced, glued on either or both
/// sides, any pair, impure contents, framing words
/// on either side of the group) — is a slot this
/// pass judges since that round: the open-phrase
/// guard in `midline_governor_before_plain_date`
/// no longer holds it, and the unwrapped line is
/// its bare-governor twin, which reads nothing.
/// The inert group at the slot is not this pass's
/// (see `governor_group_words`) and keeps serving
/// the base; a weekday-bearing base fails the
/// pass's plain-date test as at every mid-line
/// slot, so that family keeps its standing
/// refusal with its record.
fn unwrapped_trailing_or_midline_governor_group(text: &str, today: SimpleDate) -> Option<String> {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return None;
    }
    // The trailing arm's slot judgment, made once:
    // the run closing the line stands after a plain
    // date that is not a Sunday-after base.
    let trailing_run: Vec<(usize, usize)> = match trailing_group_run(text) {
        Some((run, run_start)) => {
            let prefix = &text[..run_start];
            match last_plain_date_in(prefix, today) {
                Some(date) if !sunday_after_consumed_bases(prefix, today).contains(&date) => run,
                _ => Vec::new(),
            }
        }
        None => Vec::new(),
    };
    let mut unwraps: Vec<(usize, usize, Vec<String>)> = Vec::new();
    for (idx, &(start, end)) in spans.iter().enumerate() {
        let Some(words) = governor_group_words(&text[start + 1..end - 1]) else {
            continue;
        };
        if trailing_run.contains(&(start, end))
            || midline_governor_before_plain_date(text, &spans, idx, today)
        {
            unwraps.push((start, end, words));
        }
    }
    apply_governor_unwraps(text, &unwraps)
}

/// The group sequence standing at the leading slot
/// of `text` (round 37, R37-D, F7): the framing
/// garnish stack aside, two or more top-level
/// groups standing at the line's head, separated
/// only by whitespace, commas, and the sentence
/// separators ";" "." ":" — glued, spaced, and
/// comma-separated alike, the separations the lone
/// forms allow (round 38, R38-B, F2: the lone
/// forms judge a group followed by one of those
/// marks, "(Monday); November 1, 2026", so a mark
/// standing between the groups, or between the
/// last group and the date, does not end the
/// walk). A typographic closing quote (U+2019)
/// standing as a group's suffix is taken off with
/// the separators, as the lone forms peel it
/// ("(Monday)’ November 1, 2026" is judged).
/// Round 39 (R39-C, F4, arms (a) and (e)): the
/// suffix set the trims below take off gains the
/// three marks the lone forms also judge a group
/// wearing — "!", "?", and U+201D ("(Monday)!
/// November 1, 2026" is judged) — at every
/// position the trims stand: between the groups,
/// after the last group, and stacked, the trims
/// repeating over the set so a stack peels as the
/// lone form's tail peels it ("(Monday)(notes)!’
/// November 1, 2026" is judged).
/// Returns the groups' inner texts in written
/// order and the text standing after the sequence
/// (its own leading separators taken off). A
/// single group is the lone form, judged by the
/// standing readers, and is not a sequence here; a
/// bare word standing where a group would continue
/// the sequence ends it, so the R5-D adjacency
/// doctrine — a bare word between a weekday group
/// and its date leaves the weekday ordinary text —
/// is untouched by construction.
fn leading_group_sequence(text: &str) -> Option<(Vec<String>, &str)> {
    let mut rest = skip_framing_garnish(text.trim_start());
    let mut inners: Vec<String> = Vec::new();
    loop {
        // Round 40 (R40-G, F8): the trims below gain
        // the witnessed out-of-set suffixes — the
        // ellipsis U+2026 and the sequence-tail set
        // "~", "*", "#", "-", "—" (U+2014) — which
        // stood in no reader's set, so a sequence
        // wearing one ("(Monday)(notes)~ November 1,
        // 2026", "(Monday)… (notes) November 1,
        // 2026") was no sequence at all and its
        // weekday went unjudged, while the same
        // sequence wearing a standing mark ("..",
        // "!!", "??", "!?", ":;") judged with the
        // record. The marks are garnish dress here
        // as the standing marks are: between the
        // groups and after the last group alike.
        rest = rest.trim_start_matches([
            ' ', '\t', ',', ';', '.', ':', '!', '?', '\u{201d}', '\u{2019}', '\u{2026}', '~', '*',
            '#', '-', '\u{2014}',
        ]);
        if !matches!(rest.chars().next(), Some('(' | '[' | '{')) {
            break;
        }
        let Some((start, end)) = first_balanced_span(rest) else {
            break;
        };
        if start != 0 {
            break;
        }
        let inner = &rest[start + 1..end - 1];
        inners.push(inner.to_string());
        rest = &rest[end..];
    }
    if inners.len() < 2 {
        return None;
    }
    let rest = rest.trim_start_matches([
        ' ', '\t', ',', ';', '.', ':', '!', '?', '\u{201d}', '\u{2019}', '\u{2026}', '~', '*', '#',
        '-', '\u{2014}',
    ]);
    Some((inners, rest))
}

/// The group sequence standing at the trailing
/// slot of `text` (round 37, R37-D, F7): two or
/// more top-level groups closing the line —
/// sentence punctuation and a trailing separator
/// token after the last group aside, as the
/// standing trailing readers peel them — separated
/// only by whitespace, commas, and the sentence
/// separators ";" "." ":" the lone forms allow,
/// in any spacing (round 38, R38-B, F2), with a
/// group's suffix marks standing in the gaps as at
/// the leading walk (round 39, R39-C, F4, arm
/// (a): U+2019 since R38-B, and "!", "?", U+201D
/// now — "(Monday)! (notes)" is one run). Returns
/// the run's inner texts in written order and the
/// text standing before the run (its own trailing
/// separators taken off). Groups standing
/// elsewhere in the line are not the run, and a
/// single trailing group is the lone form — with
/// one exception (round 39, R39-C, F4, arm (c)):
/// a single group standing behind a separator the
/// prefix trim takes off ("November 1, 2026
/// ;(Monday)"), the line's only group, is walked
/// here, because the lone readers are disarmed by
/// that separator token and the unseparated twin
/// is judged.
fn trailing_group_sequence(text: &str) -> Option<(Vec<String>, &str)> {
    // The suffix layers the sequence may stand in
    // front of, peeled as in
    // `spaced_trailing_weekday_garnish`: sentence
    // punctuation, then one separator token,
    // alternating until neither peels. Round 38
    // (R38-B, F2): the peel set gains the
    // typographic closing quote U+2019, which the
    // lone readers' trim sets have carried since
    // R35-C/R37-E — its pair U+201D was already
    // here, and the lone trailing form wearing
    // U+2019 ("November 1, 2026 (Monday)’") is
    // judged.
    // Round 40 (R40-G, F8): the peel set gains the
    // ellipsis U+2026 — a sequence closing the
    // line in front of one ("November 1, 2026
    // (Monday)(notes)…") is the sequence it names,
    // so its weekday groups are judged instead of
    // the date serving in silence.
    // Round 41 (R41-E, F6): the peel set gains the
    // rest of the R40-G sequence-tail set — "~",
    // "*", "#", "-", "—" (U+2014) — as at
    // `trailing_group_run`: a sequence closing the
    // line in front of one ("November 1, 2026
    // (Monday)(notes)~") is the sequence it names,
    // and the spaced form ("(Monday)(notes) ~") is
    // a suffix token of the same set, taken with
    // the standing separator tokens below.
    let mut end = text.len();
    loop {
        let peeled = text[..end]
            .trim_end_matches([
                '.', ',', ';', ':', '!', '?', '"', '\'', '\u{201d}', '\u{2019}', '\u{2026}', '~',
                '*', '#', '-', '\u{2014}',
            ])
            .len();
        if peeled != end {
            end = peeled;
            continue;
        }
        let core = &text[..end];
        let Some((ws, wc)) = core.char_indices().rev().find(|(_, c)| c.is_whitespace()) else {
            break;
        };
        let token = &core[ws + wc.len_utf8()..];
        if !token.is_empty()
            && token
                .chars()
                .all(|c| matches!(c, '*' | '|' | '~' | '#' | '-' | '\u{2014}'))
        {
            end = ws;
            continue;
        }
        break;
    }
    let core = text[..end].trim_end();
    let spans = balanced_spans(core);
    if spans.is_empty() {
        return None;
    }
    let last = spans.len() - 1;
    if spans[last].1 != core.len() {
        return None;
    }
    // The run is the suffix of the span list whose
    // members stand against each other with only
    // whitespace, commas, and the sentence
    // separators ";" "." ":" between them (a glued
    // pair's gap is empty, and stands); a U+2019
    // standing in a gap is the preceding group's
    // suffix, peeled as at the sequence's end
    // (round 38, R38-B, F2). Round 39 (R39-C, F4,
    // arm (a)): "!", "?", and U+201D stand in a gap
    // the same way — each is the preceding group's
    // suffix, as the lone forms judge it.
    let mut run_first = last;
    while run_first > 0 {
        let gap = &core[spans[run_first - 1].1..spans[run_first].0];
        if !gap.chars().all(|c| {
            matches!(
                c,
                ',' | ';' | '.' | ':' | '!' | '?' | '\u{201d}' | '\u{2019}'
            ) || c.is_whitespace()
        }) {
            break;
        }
        run_first -= 1;
    }
    // Round 39 (R39-C, F4, arm (c)): the prefix's
    // own trailing separators come off with its
    // whitespace and commas — a separator standing
    // between the date and the run ("November 1,
    // 2026 ;(Monday)(notes)") is the walk's head
    // dress, not part of the date, exactly as a
    // separator at the leading walk's head is not
    // part of its first group. Before this the
    // prefix kept its trailing separator and
    // "November 1, 2026 ;" never parsed, so the
    // run behind it went unjudged.
    let prefix_raw = &core[..spans[run_first].0];
    let prefix = prefix_raw.trim_end_matches([' ', '\t', ',', ';', '.', ':']);
    if last - run_first + 1 < 2 {
        // The single-group exception stated in the
        // walk's doc: the group is the line's only
        // group, and a separator stood between it
        // and the prefix the trim just took off.
        // A single group with other groups elsewhere
        // in the line keeps the lone form's standing
        // reading — its unseparated twin ("(notes)
        // November 1, 2026 (Monday)") serves, and so
        // does the separated form.
        let separator_came_off = prefix_raw[prefix.len()..]
            .chars()
            .any(|c| matches!(c, ',' | ';' | '.' | ':'));
        if !(spans.len() == 1 && separator_came_off) {
            return None;
        }
    }
    let mut inners: Vec<String> = Vec::new();
    for (start, end) in &spans[run_first..=last] {
        let inner = &core[start + 1..end - 1];
        inners.push(inner.to_string());
    }
    Some((inners, prefix))
}

/// `text` with a leading group run taken off — the
/// framing garnish stack, then every top-level
/// group standing at the head, in any separation
/// the sequence allows: the text a trailing-slot
/// judgment reads its date from when the line's
/// own head carries groups beside its date
/// ("(Sunday)(notes)November 1, 2026 (Monday)
/// (notes)" — the trailing sequence is judged
/// against the date the de-grouped head names).
fn strip_leading_group_run(text: &str) -> &str {
    let mut rest = skip_framing_garnish(text.trim_start());
    loop {
        rest = rest.trim_start_matches([
            ' ', '\t', ',', ';', '.', ':', '!', '?', '\u{201d}', '\u{2019}',
        ]);
        if !matches!(rest.chars().next(), Some('(' | '[' | '{')) {
            return rest;
        }
        match first_balanced_span(rest) {
            Some((0, end)) => rest = &rest[end..],
            _ => return rest,
        }
    }
}

/// The weekday mismatch a group sequence at the
/// leading or trailing slot names, when one does
/// (round 37, R37-D, F7 = review R37-R4): the lone
/// weekday group is judged at both slots — leading,
/// `(Monday)November 1, 2026` refuses with
/// WeekdayMismatch(Monday@2026-11-01); trailing,
/// `November 1, 2026 (Monday)` refuses with the
/// same record — but the standing readers stop at
/// one group: the framing reader asks for the date
/// directly after its weekday token, and the
/// trailing reader asks only for the line's last
/// word, so a weekday group standing with any
/// companion group beside it, in any spacing and
/// either position, was served in silence on both
/// routes ("(Monday)(notes)November 1, 2026",
/// "November 1, 2026 (notes) (Monday)"), breaking
/// R35-C's recorded claim that spaced and two-group
/// compound forms keep the weekday reading. The
/// month–day slot's readers already walk compound
/// groups (R37-C); this walk is the same step at
/// the two garnish slots. A sequence is two or
/// more groups standing at the slot, separated
/// only by whitespace, commas, and the sentence
/// separators ";" "." ":" as the lone forms allow
/// (round 38, R38-B, F2 — the walks' separator
/// and suffix sets are the lone forms' sets,
/// U+2019 in the suffix peels included; round 39,
/// R39-C, F4: the suffix sets gain "!", "?", and
/// U+201D, the head separators in front of a
/// single leading group and of a trailing run are
/// walked, and a stacked suffix peels as a stack
/// — the sets below are the lone forms' sets at
/// every boundary position, heads, gaps, tails,
/// and stacks alike); every
/// weekday group in the sequence is
/// judged on its contents against the date the
/// slot frames — leading, the date standing after
/// the sequence (words after the date are the
/// remainder, as the framing reader has them);
/// trailing, the date standing before the sequence
/// (read whole, its own leading groups stripped
/// exactly as the leading arm strips them) — and
/// the first disagreeing weekday in written order
/// records the mismatch the lone form records.
/// An agreeing weekday group names nothing here:
/// the line falls to the standing readers, which
/// consume it exactly as they consume the lone
/// agreeing group, so the agreeing arm converges
/// with its twin by the standing machinery, not by
/// this walk. Companion contents keep their own
/// standing judgments throughout: the walk adopts
/// nothing and governs nothing — a date-carrying
/// or governing companion is judged by the readers
/// that judge it today, and the weekday refusal
/// itself carries no parenthesized record, as
/// every standing weekday refusal does. A bare
/// word standing between the weekday group and the
/// date ends the sequence, so the R5-D adjacency
/// arm ("(Monday) notes November 1, 2026" reads
/// clean) is not this walk's to move.
/// The single group standing at the leading slot
/// behind head separators (round 39, R39-C, F4, arm
/// (c)): the framing garnish stack aside, one or
/// more of the walk's separators — "," ";" "." ":"
/// among the trimmed characters — stand before the
/// group, and the group stands at the trimmed
/// head. Returns the group's inner text and the
/// text standing after it (its own leading
/// separators and suffix marks taken off, as the
/// sequence walk's final trim takes them). A group
/// standing at the head with no separator before
/// it is the lone form, judged by the standing
/// readers, and is not returned here: the head
/// separators are what disarm those readers — the
/// weekday pass sets the group apart behind a
/// separator token, the framing scan will not
/// frame a weekday standing behind another token,
/// and the line falls to the readers that read a
/// mid-line weekday as ordinary text — while a
/// sequence of two or more behind the same
/// separators is already walked by
/// `leading_group_sequence`, whose head trim takes
/// the separators off.
fn leading_single_group_behind_head_separators(text: &str) -> Option<(String, &str)> {
    let mut rest = skip_framing_garnish(text.trim_start());
    let trimmed = rest.trim_start_matches([' ', '\t', ',', ';', '.', ':', '\u{2019}']);
    if !rest[..rest.len() - trimmed.len()]
        .chars()
        .any(|c| matches!(c, ',' | ';' | '.' | ':'))
    {
        return None;
    }
    rest = trimmed;
    if !matches!(rest.chars().next(), Some('(' | '[' | '{')) {
        return None;
    }
    let (start, end) = first_balanced_span(rest)?;
    if start != 0 {
        return None;
    }
    let inner = &rest[start + 1..end - 1];
    let rest = rest[end..].trim_start_matches([
        ' ', '\t', ',', ';', '.', ':', '!', '?', '\u{201d}', '\u{2019}',
    ]);
    Some((inner.to_string(), rest))
}

/// The single group standing at the leading slot
/// wearing one of the witnessed out-of-set suffix
/// marks glued to its closer (round 40, R40-G, F8):
/// "~", "*", "#", "-", or "—" (U+2014) standing
/// directly after the group, before the date the
/// group frames. Returns the group's inner text
/// and the text standing after the suffix (its own
/// separators and standing suffix marks taken off
/// with the witnessed mark, as the sequence walk's
/// trims take them). A group wearing no witnessed
/// mark glued to it is the lone form, judged by
/// the standing readers, and is not returned here:
/// the witnessed marks are what disarm those
/// readers at this slot — the shared cleaning
/// sheds the standing marks ("(Monday).. November
/// 1, 2026" is judged), but it sheds none of these
/// five, and "*" cannot join the cleaning without
/// moving the glued-weekday readings ("*Monday"
/// is a glued word, not dress on one) — so the
/// suffixed lone group went unjudged and its date
/// served in silence, while its sequence twin is
/// walked by `leading_group_sequence` above.
fn leading_single_group_wearing_suffix(text: &str) -> Option<(String, &str)> {
    let rest = skip_framing_garnish(text.trim_start());
    if !matches!(rest.chars().next(), Some('(' | '[' | '{')) {
        return None;
    }
    let (start, end) = first_balanced_span(rest)?;
    if start != 0 {
        return None;
    }
    let after = &rest[end..];
    if !matches!(
        after.chars().next(),
        Some('~' | '*' | '#' | '-' | '\u{2014}')
    ) {
        return None;
    }
    let inner = &rest[start + 1..end - 1];
    let rest = after.trim_start_matches([
        ' ', '\t', ',', ';', '.', ':', '!', '?', '\u{201d}', '\u{2019}', '\u{2026}', '~', '*', '#',
        '-', '\u{2014}',
    ]);
    Some((inner.to_string(), rest))
}

pub(super) fn group_sequence_weekday_mismatch(
    text: &str,
    today: SimpleDate,
) -> Option<WeekdayMismatch> {
    if let Some((inners, rest)) = leading_group_sequence(text) {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if let Some((date, _)) = parse_date_prefix(&tokens, today) {
            for inner in &inners {
                if let Some((named, number)) = garnish_group_weekday(inner) {
                    if date.weekday() != number {
                        return Some(WeekdayMismatch { named, date });
                    }
                }
            }
        }
    }
    // Round 39 (R39-C, F4, arm (c)): the single
    // leading group behind head separators, judged
    // as its unseparated twin is — the sequence
    // walk above takes two or more, and the lone
    // readers are disarmed by the separator token
    // (see `leading_single_group_behind_head_separators`).
    if let Some((inner, rest)) = leading_single_group_behind_head_separators(text) {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if let Some((date, _)) = parse_date_prefix(&tokens, today) {
            if let Some((named, number)) = garnish_group_weekday(&inner) {
                if date.weekday() != number {
                    return Some(WeekdayMismatch { named, date });
                }
            }
        }
    }
    // Round 40 (R40-G, F8): the single leading
    // group wearing a witnessed out-of-set suffix
    // mark, judged as its unsuffixed twin is — the
    // sequence walk above takes two or more, and
    // the lone readers are disarmed by the mark
    // (see `leading_single_group_wearing_suffix`).
    if let Some((inner, rest)) = leading_single_group_wearing_suffix(text) {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if let Some((date, _)) = parse_date_prefix(&tokens, today) {
            if let Some((named, number)) = garnish_group_weekday(&inner) {
                if date.weekday() != number {
                    return Some(WeekdayMismatch { named, date });
                }
            }
        }
    }
    if let Some((inners, prefix)) = trailing_group_sequence(text) {
        let stripped = strip_leading_group_run(prefix);
        let date =
            parse_flexible_date(prefix, today).or_else(|| parse_flexible_date(stripped, today));
        if let Some(date) = date {
            for inner in &inners {
                if let Some((named, number)) = garnish_group_weekday(inner) {
                    if date.weekday() != number {
                        return Some(WeekdayMismatch { named, date });
                    }
                }
            }
        }
    }
    None
}

/// The composed outcome of a leading company that
/// holds BOTH a governor and a weekday group, when
/// one stands (round 40, R40-E, F9 — doctrine,
/// adjudication 3 in the round-40 synthesis,
/// extending the R38-D / R39-H line): the date is
/// not served (the governor governs), and a
/// disagreeing weekday group in the company records
/// its WeekdayMismatch, exactly as the governor-free
/// leading sequence's own walk records it. An
/// agreeing company composes silently: the framed
/// date is returned with no mismatch, and the
/// caller serves nothing and records nothing.
/// Returns None when no such company stands at the
/// leading slot, leaving every other reader's
/// standing judgment untouched.
///
/// Why this judgment exists, and why it is stated
/// here rather than inherited from the readers
/// below: the leading governor unwrap
/// (`unwrapped_leading_governor_group`) rewrites
/// "(cancelled)(Monday) November 1, 2026" to
/// "cancelled (Monday) November 1, 2026" before the
/// sequence walk ever sees a sequence, and in the
/// rewritten line the weekday group is mid-line
/// behind a bare word — the one position cell
/// (a)(i) leaves unjudged — so the mid-line fallback
/// served the date both contents independently
/// condemn, silently. The adjudicated reading
/// re-opens (a)(i) ONLY in governor-companion
/// company at the leading slot: a weekday group
/// standing in a leading group sequence is judged
/// whether or not a governor group stands in the
/// same sequence. This walk therefore reads the
/// company AS a company, on the text the inner
/// reader is handed (where the wrapped and bare
/// governor dresses have already converged: a
/// leading governor group stands bared by the
/// unwrap above, and its bare twin was never
/// wrapped), in every company shape the witnesses
/// covered: governor groups and bare governor
/// phrases in any position of the run, inert and
/// date companions in any position, two- and
/// three-group companies, the sequence walks'
/// separator and suffix sets standing between the
/// groups or after any of them (arm (d): a mark
/// after a group no longer splits the company —
/// the separators are skipped as the sequence walk
/// skips them, so every mark dress takes the
/// composed outcome), and every pair, doubled,
/// mixed, and crossed (a crossed group, whose
/// balanced span never closes, is read to its
/// first closer of any pair, as the token readers
/// read it). The first disagreeing weekday group
/// in written order records, as the sequence walk
/// records.
///
/// Scope, by construction: the walk starts at the
/// line's head (the framing garnish stack aside),
/// steps over ordinary lead-in words standing
/// before the company's first item (round 41,
/// R41-B — see the extension note below), and stops
/// at the first item, once the company has begun,
/// that is neither a group nor a bare governor
/// phrase; a bare weekday or month word ends a
/// bare run, so bare weekday framing keeps its
/// standing readers. A bare run counts as the
/// company's governor only when its governor phrase
/// stands at the run's own head: a governor word
/// standing behind ordinary bare words ("notes
/// cancelled (Monday) ...") is a mid-line segment
/// governor, and this walk does not claim that
/// bare dress — while a governor GROUP's contents
/// count when a governor phrase stands at the
/// contents' own head (`governor_group_at_head`),
/// the same head window in grouped dress.
/// The company must frame a plain date directly: a
/// company standing before a phrase (a Sunday-after
/// phrase, a relative phrase) fails the plain-date
/// guard below and keeps its standing garnish
/// reading. No governor in the company, or no
/// weekday GROUP in it, is not this walk's: the
/// governor-free sequence is the sequence walk's,
/// and the lone contents are the lone readers'.
///
/// Returns the date the company frames, the
/// mismatch a disagreeing weekday group in it
/// records (None when every weekday group in it
/// agrees — the company then composes silently),
/// whether any group in the company was read
/// through the crossed-pair fallback, and whether
/// the company stood behind ordinary lead-in words
/// (the mid-line form): `date_line` needs those
/// flags, because the stray-mark pass re-dresses
/// a crossed group as bare words before the inner
/// reader sees the line, so a crossed company is
/// judged on the pre-stray text there and restored
/// only through that flag — and a mid-line company
/// is likewise judged there, because the mid-line
/// governor unwrap below bares its governor group
/// into the very bare run whose company the inner
/// readers leave unjudged (round 41, R41-B, F3).
///
/// Round 42 (R42-A, F3 arm (c)): a member of the
/// company may wear quotes — a quoted governor or
/// weekday span standing where a group stands is
/// read as its bracket twin, its contents judged
/// by the same head and garnish judgments, in
/// either order, at the head or behind lead-in
/// words. A lone quoted governor or weekday keeps
/// its standing lone readings; only the company
/// seat is this walk's.
///
/// Round 41 (R41-B, F3 — doctrine, round-41
/// synthesis adjudication 1, extending this
/// judgment): the company is judged wherever its
/// sequence stands, behind ordinary lead-in words
/// as at the head. The walk therefore steps over
/// ordinary bare words and number tokens standing
/// before the company's first item (the lead-in),
/// recording that it did so in the fourth return
/// value. The scope walls stand: a lead-in run
/// whose first word is a weekday or month word is
/// not stepped over (bare weekday framing and the
/// date's own words keep their readers), a lead-in
/// holding "after" is not stepped over (the
/// Sunday-after interior slot is a different
/// slot's), and the lone weekday group behind
/// ordinary words — no governor companion anywhere
/// in the sequence — is never claimed, because no
/// governor is seen: (a)(i) stands for it,
/// untouched. A group's governor contents count
/// toward the company only when a governor phrase
/// stands at the contents' own head (see
/// `governor_group_at_head`): the governor-
/// completion boundary the R40-E walk held for
/// bare runs holds for groups in the same way, in
/// leading and mid-line positions alike.
pub(super) fn leading_governor_company_judgment(
    text: &str,
    today: SimpleDate,
) -> Option<(SimpleDate, Option<WeekdayMismatch>, bool, bool)> {
    const SEPARATORS: [char; 10] = [
        ' ', '\t', ',', ';', '.', ':', '!', '?', '\u{201d}', '\u{2019}',
    ];
    let mut rest = skip_framing_garnish(text.trim_start());
    let mut governor_seen = false;
    let mut crossed_seen = false;
    let mut company_started = false;
    let mut behind_lead_in = false;
    let mut weekdays: Vec<(String, u32)> = Vec::new();
    loop {
        rest = rest.trim_start_matches(SEPARATORS);
        let Some(first) = rest.chars().next() else {
            break;
        };
        if !company_started && first.is_ascii_digit() {
            // An ordinary lead-in number token
            // ("123 (cancelled)(Monday) ..."): the
            // token is stepped over as lead-in, up
            // to the whitespace or group standing
            // after it.
            let tok_end = rest
                .find(|ch: char| ch.is_whitespace() || matches!(ch, '(' | '[' | '{'))
                .unwrap_or(rest.len());
            if tok_end == 0 {
                break;
            }
            behind_lead_in = true;
            rest = &rest[tok_end..];
            continue;
        }
        if matches!(first, '(' | '[' | '{') {
            let end = match first_balanced_span(rest) {
                Some((0, end)) => end,
                _ => {
                    // A crossed pair ("[cancelled)"):
                    // no balanced span closes, so the
                    // group is read to its first closer
                    // of any pair, as the token readers
                    // read such a group.
                    crossed_seen = true;
                    let Some((at, c)) = rest
                        .char_indices()
                        .find(|(_, c)| matches!(c, ')' | ']' | '}'))
                    else {
                        break;
                    };
                    at + c.len_utf8()
                }
            };
            if end < 2 {
                break;
            }
            let inner = &rest[1..end - 1];
            company_started = true;
            if governor_group_at_head(inner) {
                governor_seen = true;
            } else if let Some(weekday) = garnish_group_weekday(inner) {
                weekdays.push(weekday);
            }
            rest = &rest[end..];
            continue;
        }
        if matches!(first, '"' | '\'' | '\u{201c}' | '\u{2018}') {
            // Round 42 (R42-A, F3 arm (c)): a company
            // member wearing quotes is taken as its
            // bracket twin — the quoted span is read
            // as a group standing in the company, its
            // contents judged by the same two
            // judgments the bracket arm applies
            // (`governor_group_at_head`,
            // `garnish_group_weekday`). A lone quoted
            // governor or weekday was already judged
            // as its paren twin by the lone readers;
            // only the company walk missed the dress,
            // and the company collapsed to a silent
            // serve. The span ends at the first
            // closing-family quote, as the re-dress
            // scan reads a quoted span.
            let open_len = first.len_utf8();
            let Some((at, c)) = rest[open_len..]
                .char_indices()
                .find(|(_, c)| matches!(c, '"' | '\'' | '\u{201d}' | '\u{2019}'))
            else {
                break;
            };
            let end = open_len + at + c.len_utf8();
            let inner = &rest[open_len..open_len + at];
            company_started = true;
            if governor_group_at_head(inner) {
                governor_seen = true;
            } else if let Some(weekday) = garnish_group_weekday(inner) {
                weekdays.push(weekday);
            }
            rest = &rest[end..];
            continue;
        }
        if first.is_alphabetic() {
            // A bare governor phrase standing in the
            // run: the governor group's bared words,
            // or the bare twin's own words. The run is
            // the consecutive bare words at this
            // position; a weekday or month word after
            // the first ends it (that material is the
            // date's own framing, not the run's), and
            // a run whose first word is one is not a
            // company item at all.
            let mut scan = rest;
            let mut words: Vec<String> = Vec::new();
            let mut run_clean = true;
            loop {
                scan = scan.trim_start_matches([' ', '\t']);
                let Some(c) = scan.chars().next() else {
                    break;
                };
                if matches!(c, '(' | '[' | '{') || !c.is_alphabetic() {
                    break;
                }
                let tok_end = scan
                    .find(|ch: char| ch.is_whitespace() || matches!(ch, '(' | '[' | '{'))
                    .unwrap_or(scan.len());
                let token = &scan[..tok_end];
                let cleaned: Vec<String> = segment_tokens(&[token])
                    .into_iter()
                    .filter(|w| !w.is_empty())
                    .collect();
                if cleaned.len() != 1 {
                    run_clean = false;
                    break;
                }
                let word = cleaned[0].clone();
                if !words.is_empty()
                    && (weekday_word(&word).is_some() || word_at_is_month(&word, 0))
                {
                    break;
                }
                words.push(word);
                scan = &scan[tok_end..];
            }
            if !run_clean || words.is_empty() {
                break;
            }
            if weekday_word(&words[0]).is_some() || word_at_is_month(&words[0], 0) {
                break;
            }
            // The run carries a governor when a
            // governor phrase stands at the run's own
            // head — the pure arm (the whole run is
            // the phrase) and the governor-first
            // impure arm ("cancelled today ...") are
            // the same window read at the head. A
            // governor phrase standing later in the
            // run stands behind ordinary bare words
            // and is not this walk's (see the head
            // note): that shape is the mid-line
            // segment governor's.
            let has_governor = (1..=3usize).any(|len| {
                words
                    .get(..len)
                    .is_some_and(|p| is_governor_phrase(p) || is_cancellation_phrase(p))
            });
            if !has_governor {
                if company_started {
                    break;
                }
                // Round 41 (R41-B, F3): a bare run
                // standing before the company's first
                // item is ordinary lead-in ("notes",
                // "notes from Sue", "please note") and
                // is stepped over, so the company
                // behind it is reached as the company
                // at the head is reached. Two lead-ins
                // are not stepped over: one holding
                // "after" (the Sunday-after interior
                // slot is a different slot's, and its
                // company keeps its standing serve),
                // and one whose first word is a
                // weekday or month word (handled by
                // the break above this window). The
                // run is stepped over whole: a
                // governor phrase standing inside it
                // ("notes cancelled (Monday) ...") is
                // the mid-line segment governor's
                // shape, not this walk's — that bare
                // dress keeps its standing reading.
                if words.iter().any(|w| w == "after") {
                    break;
                }
                behind_lead_in = true;
                rest = scan;
                continue;
            }
            governor_seen = true;
            company_started = true;
            rest = scan;
            continue;
        }
        break;
    }
    if !governor_seen || weekdays.is_empty() {
        return None;
    }
    // The company frames a plain date directly (the
    // sequence walk's own shape: words after the
    // date are the remainder). A phrase standing
    // where the date would stand — its first word a
    // weekday word, or any word that is neither a
    // month word nor a digit — is not framed by the
    // company and keeps its standing reading.
    rest = rest.trim_start_matches(SEPARATORS);
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let first_cleaned = group_token(tokens.first()?);
    if weekday_word(&first_cleaned).is_some() {
        return None;
    }
    if !first_cleaned
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
        && !word_at_is_month(&first_cleaned, 0)
    {
        return None;
    }
    let (date, _) = parse_date_prefix(&tokens, today)?;
    let mismatch = weekdays
        .iter()
        .find(|(_, number)| date.weekday() != *number)
        .map(|(named, _)| WeekdayMismatch {
            named: named.clone(),
            date,
        });
    Some((date, mismatch, crossed_seen, behind_lead_in))
}

/// Whether a group's contents carry a governor
/// phrase at their own head, judged as the bare
/// run's head window judges a bared group (round
/// 41, R41-B, F3): the pure arm (the whole contents
/// are a governor phrase, the standing
/// `is_governor_phrase` / `is_cancellation_phrase`
/// judgments over the group's cleaned words), and
/// the governor-first impure arm ("cancelled
/// today" — a governor phrase standing at the
/// contents' head, in a one-to-three-word window,
/// in contents holding no date and no weekday).
/// A governor phrase standing behind other words
/// in the contents ("today cancelled", "meeting
/// cancelled") is NOT this judgment's: no walk
/// completes a governor phrase standing behind
/// other words, in either dress, at the leading
/// slot or mid-line — the boundary the round-41
/// review held in both positions. Contents
/// holding a date or a weekday are judged whole
/// by their standing readers, as in
/// `governor_group_words`.
fn governor_group_at_head(inner: &str) -> bool {
    let raw_words: Vec<&str> = inner.split_whitespace().collect();
    let words: Vec<String> = segment_tokens(&raw_words)
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return false;
    }
    if is_governor_phrase(&words) || is_cancellation_phrase(&words) {
        return true;
    }
    let holds_date_or_weekday = words.iter().any(|w| {
        word_at_is_month(w, 0) || weekday_word(w).is_some() || w.chars().any(|c| c.is_ascii_digit())
    });
    if holds_date_or_weekday {
        return false;
    }
    (1..=3usize).any(|len| {
        words
            .get(..len)
            .is_some_and(|p| is_governor_phrase(p) || is_cancellation_phrase(p))
    })
}

/// Read one of a contribution's two date lines: the
/// date when the line reads, and the mismatch record
/// when the line named a weekday its date does not
/// fall on. A weekday word speaks about the date only
/// when it directly frames it (round 4, R4-7; round
/// 5, R5-D): first, as the Sunday line has always
/// allowed, or as the line's last word after the date,
/// in the way an office writes "October 25, 2026
/// (Monday)". A weekday word anywhere else is
/// ordinary text: it records no mismatch, and a date
/// it stands directly in front of still reads. A date
/// that does not read yields None for the date, never
/// a guess.
/// Whether the token at `i` in `tokens` is the
/// weekday word of a bare phrase unit ending at
/// `i`, judged on the unglued token vector (round
/// 29, R29-A, F3 = red team RT29-T1): the unit's
/// words are read unglued, as governor phrases are
/// in `segment_tokens`, so a unit whose weekday
/// word itself wears the glue ("see you *sunday")
/// still ends on its weekday word, exactly as the
/// forms R28-C pinned (glue on the unit's earlier
/// words, its weekday clean) do. A glued weekday
/// word that ends no bare unit — the standalone
/// "*Monday" garnish before a date — is not this
/// shape and keeps its standing reading.
pub(super) fn bare_unit_weekday_at(tokens: &[&str], i: usize) -> bool {
    // Round 35 (R35-A, F1): the trim set gains the
    // brace pair, as at the weekday readers above.
    let word = tokens[i].trim_matches(['(', ')', '[', ']', '{', '}', ',', '.']);
    if weekday_word(&unglued_word(word)).is_none() {
        return false;
    }
    let words: Vec<String> = tokens[..=i]
        .iter()
        .map(|t| unglued_word(&group_token(t)))
        .collect();
    (0..=i).any(|s| bare_phrase_unit_span(&words, s).is_some_and(|span| s + span == i + 1))
}

/// `text` with a space set before a trailing
/// parenthesized weekday garnish that stands GLUED
/// to the word before it (round 33, R33-B, F2 = red
/// team RT33-2): "November 1, 2026(Monday)". The
/// spaced twin — "November 1, 2026 (Monday)" — is
/// the pinned R4-7 shape: its garnish is the line's
/// last word, so the trailing-weekday check judges
/// it and a disagreeing weekday refuses the date
/// and records the mismatch. Glued, the garnish
/// rode inside the year token, no reader ever saw
/// it as a word, and the line's trailing group was
/// later stripped as anonymous garnish: the date
/// read clean and the check was silenced. Setting
/// the one space hands the line to every reader
/// below exactly as its spaced twin is handed, so
/// the glued garnish is judged by the standing
/// machinery, word for word — an agreeing one
/// reads as spaced reads, a disagreeing one
/// refuses and records as spaced records, leading
/// weekday and all. Only this shape is re-spaced:
/// the group must close the line (sentence
/// punctuation after it aside), hold one weekday
/// word and nothing else, and stand glued — a
/// group already standing apart, a parenthesized
/// DATE, and every other group keep their text
/// byte for byte.
///
/// Round 34 (R34-C, F4 = red team RT34-2 paren
/// half + review MINOR-2 + RT34-5): the gate
/// stopped one shape short, so the suffix it sees
/// past is widened — still the one gate, still
/// handing the standing machinery the spaced twin:
/// - Extra closing marks after the group
///   ("2026(Monday))", "2026(Monday)]"): the spaced
///   machinery's token trim already strips the
///   extra mark; the gate's inner-word test saw
///   "Monday)" and never fired.
/// - A doubled garnish ("2026((Monday))"): the
///   group the gate must find is the OUTER pair;
///   its contents, parentheses aside, are still
///   the one weekday word.
/// - A trailing separator token ("2026(Monday) *",
///   "… |" — the format's own separators): the
///   spaced twin is judged because the
///   trailing-weekday scan skips non-alphanumeric
///   tokens when it looks for the line's last word.
/// - Trailing punctuation ("2026(Monday);", and
///   ":", "!", "?", the closing quote): peeled
///   with the sentence punctuation here, and the
///   trailing-word trim set below and in `clean`
///   extends to the same marks, so the garnish
///   check fires in both forms.
///
/// The re-spaced line keeps the group and every
/// suffix byte exactly as written; only the one
/// space is set. A group holding anything but one
/// weekday word — "(notes)", "(November)", "(next
/// week)", "(Monday morning)", "(2027)",
/// "(Sundae)", "(Mondays)", a parenthesized DATE —
/// is never re-spaced by THIS (trailing) pass, in
/// any of these shapes. (Round 36, R36-C: at the
/// month–day and leading slots a group of any
/// contents IS re-spaced, by the date-slot pass
/// below — the two passes state different patterns
/// for different slots, and neither statement
/// covers the other's.)
///
/// Round 35 (R35-C, F3 = review R35-R3 = red team
/// RT35-4): the gate above judged only the TRAILING
/// slot. At the leading slot ("(Monday)November 1,
/// 2026") and the month–day slot ("November(Monday)
/// 1, 2026") a glued weekday group was invisible to
/// every reader in all three pairs, while its spaced
/// twin was judged — leading, the spaced form refuses
/// and records the mismatch; month–day, the spaced
/// form reads no date at all, agreeing or not, as
/// the bare word there does. The coordinator's
/// ruling is the R33-B/R34-C doctrine: spacing is
/// not semantics, so a glued weekday group judges
/// exactly as its spaced twin at every slot. The
/// gate therefore runs in two passes over the line:
/// `respaced_glued_weekday_groups` sets a space on
/// every glued side of every standalone one-weekday
/// group wherever it stands (the leading slot's
/// glue is on the group's right, against the month
/// word; the month–day slot's is on its left), and
/// the trailing pass below then judges any shape
/// the first pass's plainer pattern does not cover
/// (its suffix layers, a foreign extra closer). The
/// day–year slot needs no pass: its glued and spaced
/// forms already read alike, and the first pass
/// hands it its spaced text, which reads the same.
///
/// Round 35 (R35-C, F7 = red team RT35-5): the peel
/// set below learns the apostrophe and the
/// typographic closing quote — a disagreeing garnish
/// followed by "'" or "”" was judged in neither
/// form, because the peel stopped before both marks
/// and the weekday readers' sentence-mark trim sets
/// (`clean`, the trailing scans here and in
/// `sunday_after_base`) stopped at the same place;
/// those sets learn both marks with this one, so
/// the garnish check fires in both forms exactly as
/// it does for the standing marks.
///
/// Round 36 (R36-C, F1 + F10 + F9): the gate stopped
/// one pattern short again. Past the one-weekday
/// single-pair pattern, glued and spaced split at
/// the month–day slot for EVERY standalone group —
/// a governing group ("November(cancelled) 1, 2026"
/// served the cancelled date live; the spaced twin
/// and the bare twin read nothing), a multi-word
/// governor ("November[called off] 1, 2026"), a
/// date group ("November(November 8, 2026) 1, 2026"
/// read November 1 with a record naming November 8
/// where the spaced twin reads November 8 clean),
/// and inert groups alike ("November(notes) 1,
/// 2026" read where "November (notes) 1, 2026"
/// reads nothing, identically in all three pairs) —
/// and a mixed-pair wrapper escaped at both the
/// month–day slot and the leading slot
/// ("November([Monday]) 1, 2026" and
/// "([Monday])November 1, 2026" both served
/// November 1 in silence, where every spaced twin
/// refuses or records). The adjudicated reading
/// for the whole family is the standing one:
/// spacing is not semantics — the glued form takes
/// the spaced form's reading. The gate therefore
/// gains a pass over the line's top-level groups
/// (`respaced_glued_groups_at_date_slots`): a
/// standalone group of ANY contents standing at
/// the month–day slot (glued against the month
/// word on its left, a day number standing after
/// it) or at the leading slot (the line's first
/// group, glued against the month word on its
/// right) is re-spaced on its glued side, whatever
/// it holds and however it is wrapped — mixed
/// pairs and doubled pairs included, because the
/// pass reads the group's balanced span, not its
/// edges. And the trailing pass below learns the
/// mixed-pair wrapper's shape (F9): a trailing
/// garnish in a mixed pair ("November 1, 2026
/// ([Monday])") was judged in neither form — the
/// candidate scan found only the wrapper's inner
/// pair and set its space inside the wrapper,
/// breaking the very token the trailing readers
/// judge — where its single-pair and doubled
/// same-pair twins are judged; the candidate's
/// contents now shed opening and closing marks of
/// any pair, so the wrapper itself is the
/// candidate, a spaced wrapper needs no space and
/// stands as written for the standing readers to
/// judge, and a glued one is re-spaced at its
/// outer edge.
///
/// Round 37 (R37-C, F5 + F6): the date-slot pass
/// stopped one pattern short once more — a compound
/// of two groups glued together at the month–day
/// slot qualified on NEITHER span (the first is
/// followed by a group, not a day digit; the second
/// is preceded by a group, not a month word), so
/// the compound was never re-spaced: governing
/// contents in either position served the date live
/// ("November(cancelled)(notes) 1, 2026" and its
/// reverse read November 1 with no record; the
/// spaced twins read nothing), a mixed weekday
/// wrapper served silently, a date group first read
/// the outer date with a record where the spaced
/// twin reads the group's date clean, and inert
/// compounds read November 1 glued, nothing spaced.
/// The pass now states the slot for a chain of
/// glued spans as a unit — month word before the
/// chain's first group, day number after its last —
/// and sets a space on every glued side of the
/// chain, so the compound takes its spaced reading
/// exactly as the single group at the slot does,
/// for a chain of any length.
///
/// Round 38 (R38-E, F5 + F6): the chain stopped at
/// a "," or ";" separator between its groups — the
/// glued separator chain ("November(cancelled),
/// (notes) 1, 2026") and the lone group wearing the
/// separator before its day ("November(cancelled),
/// 1, 2026") escaped the pass while their spaced
/// and half-glued twins were judged; the pass now
/// states the chain for separator links and the
/// day past separator dress (see
/// `respaced_glued_groups_at_date_slots`).
pub(super) fn spaced_glued_weekday_garnish(text: &str) -> Option<String> {
    let at_slots = respaced_glued_groups_at_date_slots(text);
    let stage0: &str = at_slots.as_deref().unwrap_or(text);
    let mid = respaced_glued_weekday_groups(stage0);
    let stage: &str = mid.as_deref().unwrap_or(stage0);
    match spaced_trailing_weekday_garnish(stage) {
        Some(spaced) => Some(spaced),
        None => match mid {
            Some(spaced) => Some(spaced),
            None => at_slots,
        },
    }
}

/// The inserts applied to `text`, in ascending
/// order: a space set at each position, every other
/// byte as written.
fn apply_space_inserts(text: &str, inserts: &[usize]) -> String {
    let mut spaced = String::with_capacity(text.len() + inserts.len());
    let mut at = 0;
    for p in inserts {
        spaced.push_str(&text[at..*p]);
        spaced.push(' ');
        at = *p;
    }
    spaced.push_str(&text[at..]);
    spaced
}

/// Whether the alphabetic word standing at `at` in
/// `text` (the run of letters there) is a month
/// name or one of its standing abbreviations — the
/// same table the flexible reader reads
/// (`month_number` in ob-model, restated here
/// because this crate's readers name the word, not
/// its number).
fn word_at_is_month(text: &str, at: usize) -> bool {
    let run: String = text[at..]
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    matches!(
        run.to_ascii_lowercase().as_str(),
        "jan"
            | "january"
            | "feb"
            | "february"
            | "mar"
            | "march"
            | "apr"
            | "april"
            | "may"
            | "jun"
            | "june"
            | "jul"
            | "july"
            | "aug"
            | "august"
            | "sep"
            | "sept"
            | "september"
            | "oct"
            | "october"
            | "nov"
            | "november"
            | "dec"
            | "december"
    )
}

/// Whether the word ending immediately before byte
/// `before` in `text` is a month name: the letter
/// run ending there (a trailing "." the abbreviation
/// may wear is stepped over first), read whole — a
/// longer run is a longer word, not a month glued
/// inside one.
fn word_before_is_month(text: &str, before: usize) -> bool {
    let mut end = before;
    while end > 0 && matches!(text.as_bytes()[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    if end > 0 && text.as_bytes()[end - 1] == b'.' {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && text.as_bytes()[start - 1].is_ascii_alphabetic() {
        start -= 1;
    }
    if start == end {
        return false;
    }
    word_at_is_month(text, start)
}

/// Whether a day-of-month number stands at the
/// first non-whitespace byte at or after `at`: one
/// or two digits naming 1..=31, followed by a comma,
/// whitespace, or the line's end.
fn day_number_stands_after(text: &str, at: usize) -> bool {
    let rest = text[at..].trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() > 2 {
        return false;
    }
    let Ok(day) = digits.parse::<u32>() else {
        return false;
    };
    if !(1..=31).contains(&day) {
        return false;
    }
    match rest[digits.len()..].chars().next() {
        None => true,
        Some(c) => c == ',' || c.is_whitespace(),
    }
}

/// The byte position where a day-of-month number
/// stands at or after `at`, past the separator
/// dress a group at the month–day slot may wear
/// between itself and its day (round 38, R38-E,
/// F5 + F6): whitespace, then at most one "," or
/// ";" separator, then whitespace again — the
/// dressed forms of `day_number_stands_after`, with
/// the day itself under the same test (one or two
/// digits naming 1..=31, followed by a comma,
/// whitespace, or the line's end). Round 39
/// (R39-D, F5 + F6): the dress is the full set —
/// one or a run of "," ";" "." ":" separators,
/// with whatever whitespace the sender set around
/// or between them ("November(cancelled). 1,
/// 2026", "November(cancelled): 1, 2026", and
/// "November(cancelled),; 1, 2026" wear it, as the
/// "," / ";" dresses have since R38-E). Returns
/// the day's first byte, so the caller can tell a
/// day standing glued (against a group or against
/// the separator itself) from one standing apart.
fn dressed_day_start(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = at;
    loop {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if i < bytes.len() && matches!(bytes[i], b',' | b';' | b'.' | b':') {
            i += 1;
        } else {
            break;
        }
    }
    if day_number_stands_after(text, i) {
        Some(i)
    } else {
        None
    }
}

/// Whether the gap between two top-level spans is
/// a chain link (round 38, R38-E, F5): nothing at
/// all (the spans stand immediately against each
/// other, the R37-C glued chain), or one "," or ";"
/// separator with whatever whitespace the sender
/// set around it — the separator-joined chain.
/// Round 39 (R39-D, F5 + F6): the link is the
/// full set — one or a run of "," ";" "." ":"
/// separators, with whatever whitespace the
/// sender set around or between them
/// ("November(cancelled).(notes) 1, 2026",
/// "November(cancelled):(notes) 1, 2026", and the
/// ",," / ";;" / ",;" runs are links, as the ","
/// and ";" links have been since R38-E). A gap of
/// whitespace alone is no link: groups standing
/// apart are the spaced form already, and keep
/// the per-span reading.
fn chain_gap_is_link(gap: &str) -> bool {
    gap.is_empty()
        || (!gap.trim().is_empty()
            && gap
                .chars()
                .all(|c| matches!(c, ',' | ';' | '.' | ':') || c.is_whitespace()))
}

/// `text` with a space set on every glued side of
/// every standalone group standing at the month–day
/// slot or the leading slot, whatever the group
/// holds (round 36, R36-C, F1 + F10): the pass
/// reads the line's top-level balanced spans, so a
/// mixed-pair wrapper and a doubled pair are the
/// group they stand as, and the contents — one
/// word, several words, a whole date — never enter
/// the pattern. The month–day slot is stated by
/// its neighbours: a month word ending immediately
/// before the group and a day number standing after
/// it. The leading slot is the line's first group
/// (nothing but space before it) glued against a
/// month word on its right, the space setting after
/// any peel mark the group wears there, exactly as
/// the weekday pass sets it. A group at any other
/// slot — the day–year slot, the trailing slot, a
/// group standing in running words — keeps its
/// bytes from this pass, as do groups standing
/// apart on the side in question.
///
/// Round 37 (R37-C, F5 + F6): the per-span
/// neighbours above are the single group's, and a
/// compound of two groups glued together at the
/// month–day slot satisfied neither span's: the
/// first is followed by a group, not a day digit,
/// and the second is preceded by a group, not a
/// month word — so the compound was never
/// re-spaced, and governing contents served the
/// date live, a date group first flipped the
/// reading to the outer date with a record, and
/// inert contents read a date their spaced twins
/// refuse. The slot is therefore also stated for a
/// CHAIN of glued spans — two or more top-level
/// spans standing immediately against each other —
/// as a unit: a month word ending immediately
/// before the chain's first group and a day number
/// standing after the chain's last group, with a
/// space set on every glued side of the chain (its
/// head, between its groups, and at its tail when
/// the day number itself stands glued there). The
/// chain rule keys on the same two neighbours as
/// the single group's, so a chain of any length
/// takes its spaced reading the same way; a chain
/// standing anywhere else, and groups standing
/// apart inside a would-be chain (the half-glued
/// form), keep the per-span reading below.
///
/// Round 38 (R38-E, F5 MAJOR + F6 MINOR): the
/// chain rule stopped one pattern short again —
/// it keyed on spans standing immediately against
/// each other, so a "," or ";" separator between
/// the groups defeated it, and neither span then
/// qualified alone (the first is followed by the
/// separator, not a day digit; the second is
/// preceded by the separator, not a month word).
/// Glued at the separator specifically, a chain
/// was never re-spaced: governing contents served
/// the date live in every pair
/// ("November(cancelled),(notes) 1, 2026" read
/// November 1; the spaced twin and every
/// half-glued form read nothing), inert contents
/// read a date their spaced twins refuse
/// ("November(notes),(choir) 1, 2026", the
/// three-group and semicolon chains), and a
/// weekday group in a comma chain stood in the
/// same silence as its spaced twin only by
/// accident of neither being read. The single
/// group escaped the same way wearing the
/// separator as dress before its day
/// ("November(cancelled), 1, 2026" and
/// "November(cancelled) , 1, 2026" served, where
/// "November (cancelled), 1, 2026" reads
/// nothing). The slot is therefore stated for a
/// chain whose links may be a separator as well
/// as bare adjacency (`chain_gap_is_link`), and
/// the day after the slot's last group — or after
/// a lone group — may stand past the same
/// separator dress (`dressed_day_start`). Spaces
/// are set only on glued sides: the chain's head
/// against the month word, each later group
/// against the span or separator it stands glued
/// to, and the day against the group or separator
/// it stands glued to — so the re-spaced line is
/// a spaced twin the standing readers judge, byte
/// for byte in shape. Comma spacing in this
/// family is dress and flips nothing.
///
/// Round 39 (R39-D, F5 MAJOR + F6 MINOR): the
/// R38-E statement stopped one separator set
/// short again — the sentence separators "." and
/// ":" were not chain links, runs of separators
/// were not links in either set, and a lone group
/// wearing "." or ":" (or a run) as dress before
/// its day escaped the same way ("November
/// (cancelled).(notes) 1, 2026" and the ":" form
/// served November 1 live where the spaced twin
/// governs; "November(cancelled),,(notes) 1,
/// 2026", ";;", and ",;" runs served the same
/// way; "November(cancelled). 1, 2026",
/// "November(cancelled): 1, 2026", and
/// "November(cancelled),; 1, 2026" served where
/// the "," / ";" dresses have governed since
/// R38-E; the inert chains "November(notes).
/// (choir) 1, 2026" and its three-group form read
/// a date their spaced twins refuse). The slot
/// therefore takes the full set in both helpers:
/// `chain_gap_is_link` admits bare adjacency, or
/// one or a run of "," ";" "." ":", and
/// `dressed_day_start` admits the same run as
/// dress before the day. The weekday arms at
/// this site stand as they stood: the comma
/// chains and the spaced forms alike read
/// nothing with no record, and the new links
/// take the same reading (their re-spaced lines
/// are the spaced forms).
fn respaced_glued_groups_at_date_slots(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut inserts: Vec<usize> = Vec::new();
    let spans = balanced_spans(text);
    let mut in_chain = vec![false; spans.len()];
    // The compound chains (round 37, R37-C): runs
    // of two or more spans standing immediately
    // against each other, judged at the month–day
    // slot as a unit. Round 38 (R38-E): a run's
    // links may also be one "," or ";" separator
    // (`chain_gap_is_link`) — the separator-joined
    // chain is the same compound at the same slot —
    // and the day after the run may stand past the
    // same separator dress (`dressed_day_start`).
    // Round 39 (R39-D): the links and the dress
    // take the full set — one or a run of ","
    // ";" "." ":" as well as bare adjacency.
    // Spaces are set on glued sides only, as the
    // function's head note states.
    let mut chain_at = 0;
    while chain_at < spans.len() {
        let mut chain_end = chain_at;
        while chain_end + 1 < spans.len()
            && chain_gap_is_link(&text[spans[chain_end].1..spans[chain_end + 1].0])
        {
            chain_end += 1;
        }
        if chain_end > chain_at {
            let (start, _) = spans[chain_at];
            let (_, end) = spans[chain_end];
            let left_glued = start > 0
                && !text[..start]
                    .chars()
                    .next_back()
                    .expect("a byte stands before")
                    .is_whitespace();
            if word_before_is_month(text, start) {
                if let Some(day_start) = dressed_day_start(text, end) {
                    if left_glued {
                        inserts.push(start);
                    }
                    for span in &spans[chain_at + 1..=chain_end] {
                        let span_start = span.0;
                        if !text[..span_start]
                            .chars()
                            .next_back()
                            .expect("a byte stands before")
                            .is_whitespace()
                        {
                            inserts.push(span_start);
                        }
                    }
                    if !text[..day_start]
                        .chars()
                        .next_back()
                        .expect("a byte stands before")
                        .is_whitespace()
                    {
                        inserts.push(day_start);
                    }
                    for taken in in_chain.iter_mut().take(chain_end + 1).skip(chain_at) {
                        *taken = true;
                    }
                }
            }
        }
        chain_at = chain_end + 1;
    }
    for (span_at, (start, end)) in spans.iter().enumerate() {
        if in_chain[span_at] {
            continue;
        }
        let (start, end) = (*start, *end);
        // The position on the group's right where a
        // glued word would stand: past any peel
        // marks the group wears there.
        let mut k = end;
        while k < bytes.len() {
            let c = text[k..].chars().next().expect("a byte stands after");
            if matches!(
                c,
                '.' | ',' | ';' | ':' | '!' | '?' | '"' | '\'' | '\u{201d}'
            ) {
                k += c.len_utf8();
            } else {
                break;
            }
        }
        let right_glued_at = if k < bytes.len()
            && text[k..]
                .chars()
                .next()
                .expect("a byte stands after")
                .is_alphanumeric()
        {
            Some(k)
        } else {
            None
        };
        let left_glued = start > 0
            && !text[..start]
                .chars()
                .next_back()
                .expect("a byte stands before")
                .is_whitespace();
        // The month–day slot. Round 38 (R38-E):
        // the day may stand past a "," or ";"
        // separator dress after the group
        // (`dressed_day_start`) — the lone group
        // wearing the separator before its day is
        // the chain family's single-group form, and
        // its space sets at the same glued sides.
        // Round 39 (R39-D): the dress is the full
        // set — one or a run of "," ";" "." ":".
        if word_before_is_month(text, start) {
            if let Some(day_start) = dressed_day_start(text, end) {
                if left_glued {
                    inserts.push(start);
                }
                if !text[..day_start]
                    .chars()
                    .next_back()
                    .expect("a byte stands before")
                    .is_whitespace()
                {
                    inserts.push(day_start);
                }
                continue;
            }
        }
        // The leading slot.
        if text[..start].trim().is_empty() {
            if let Some(at) = right_glued_at {
                if word_at_is_month(text, at) {
                    inserts.push(at);
                }
            }
        }
    }
    if inserts.is_empty() {
        return None;
    }
    inserts.sort_unstable();
    inserts.dedup();
    Some(apply_space_inserts(text, &inserts))
}

/// `text` with a space set on every glued side of
/// every standalone group holding one weekday word
/// and nothing else, wherever the group stands in
/// the line (round 35, R35-C, F3): the leading slot
/// ("(Monday)November 1, 2026" — the glue is on the
/// group's right, against the word after it —
/// wearing a peel mark of its own there,
/// "(Monday)'November 1, 2026", the space sets
/// after the mark) and
/// the month–day slot ("November(Monday) 1, 2026" —
/// the glue is on its left) are the witnessed slots;
/// the trailing slot is this pass's degenerate case
/// and keeps the reading the trailing gate has
/// always given it. A group is standalone, FOR
/// THIS PASS, when it does not stand immediately
/// inside another group's opener — the content of
/// a mixed-pair wrapper ("([Monday])") is not
/// re-spaced here, because this pass reads one
/// weekday word at a group's own edges and the
/// wrapper's inner group is not that group. Round
/// 36 (R36-C, F1) corrected the stronger claim an
/// earlier statement of this rule carried — that
/// the wrapper's forms "have always read" alike:
/// they had not. At the month–day and leading
/// slots the glued wrapper was erased and the date
/// read, where the spaced wrapper interrupted or
/// refused. The wrapper itself is the earlier
/// date-slot pass's business
/// (`respaced_glued_groups_at_date_slots`), which
/// re-spaces the wrapper's balanced span at those
/// slots; at the trailing slot the trailing pass
/// below judges it (R36-C, F9). The group's own shape is the
/// trailing gate's, stated flat: some openers of one
/// pair, one weekday word (a trailing mark
/// `weekday_word` itself trims may follow it), some
/// closing marks. A side is glued when the byte
/// beside it is not whitespace — on the left, any
/// such byte; on the right, a letter or digit, the
/// word the group stands glued against (a closing
/// mark, a separator, or sentence punctuation
/// standing after the group is suffix in both forms
/// and keeps its bytes). Only spaces are set; the
/// group and every other byte stay as written.
fn respaced_glued_weekday_groups(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut inserts: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let opener = bytes[i];
        if !matches!(opener, b'(' | b'[' | b'{') {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < bytes.len() && bytes[j] == opener {
            j += 1;
        }
        let word_start = j;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j == word_start || weekday_word(&text[word_start..j]).is_none() {
            // Round 40 (R40-C, F2): the failure is
            // the whole opener run's — every
            // position inside the run shares the
            // run's end, the word standing there,
            // and the closers after it, so each
            // would fail this same test — and the
            // scan resumes at the run's end, not
            // one byte in. A run of openers whose
            // word is no weekday (a nested date:
            // "(((…November…)))") otherwise
            // restarts the run scan at every
            // position, one whole-run scan per
            // nesting layer.
            i = word_start;
            continue;
        }
        while j < bytes.len() && matches!(bytes[j], b'.' | b',' | b':' | b';' | b'/') {
            j += 1;
        }
        let closers_start = j;
        while j < bytes.len() && matches!(bytes[j], b')' | b']' | b'}') {
            j += 1;
        }
        if j == closers_start {
            i = word_start;
            continue;
        }
        if i > 0 {
            let prev = text[..i].chars().next_back().expect("a byte stands before");
            if matches!(prev, '(' | '[' | '{') {
                i = j;
                continue;
            }
            if !prev.is_whitespace() {
                inserts.push(i);
            }
        }
        if j < bytes.len() {
            // The word the group stands glued against
            // may itself wear a peel mark the garnish
            // carries — "(Monday)'November 1, 2026" is
            // the leading group, its apostrophe, and
            // the month word in one token run; the
            // space belongs after the mark, where the
            // spaced twin "(Monday)' November 1, 2026"
            // sets it.
            let mut k = j;
            while k < bytes.len() {
                let c = text[k..].chars().next().expect("a byte stands after");
                if matches!(
                    c,
                    '.' | ',' | ';' | ':' | '!' | '?' | '"' | '\'' | '\u{201d}'
                ) {
                    k += c.len_utf8();
                } else {
                    break;
                }
            }
            if k < bytes.len() {
                let next = text[k..].chars().next().expect("a byte stands after");
                if next.is_alphanumeric() {
                    inserts.push(k);
                }
            }
        }
        i = j;
    }
    if inserts.is_empty() {
        return None;
    }
    let mut spaced = String::with_capacity(text.len() + inserts.len());
    let mut at = 0;
    for p in inserts {
        spaced.push_str(&text[at..p]);
        spaced.push(' ');
        at = p;
    }
    spaced.push_str(&text[at..]);
    Some(spaced)
}

/// The trailing pass of the gate above: `text` with
/// a space set before a trailing one-weekday group
/// that stands glued to the word before it, seeing
/// past the suffix layers the round-34 widening
/// named (extra closing marks, a doubled garnish, a
/// trailing separator token, trailing punctuation).
fn spaced_trailing_weekday_garnish(text: &str) -> Option<String> {
    // The suffix layers the group may stand in
    // front of, peeled from the line's end: sentence
    // punctuation, then one separator token,
    // alternating until neither peels — "(Monday);"
    // sheds its ";", "(Monday) *" sheds its token,
    // "(Monday) *;" sheds both.
    let mut end = text.len();
    loop {
        // Round 35 (R35-C, F7): the peel set gains
        // the apostrophe and the typographic closing
        // quote, as the trim sets in the garnish path
        // do — a garnish followed by either mark is
        // peeled to here exactly as one followed by
        // ";" or the closing quote already is.
        // Round 40 (R40-G, F8): the peel set gains
        // the ellipsis U+2026, as the trailing
        // walks' peels do — a glued garnish followed
        // by one is peeled to here exactly as one
        // followed by ";" already is.
        // Round 41 (R41-E, F6): the peel set gains
        // the rest of the R40-G sequence-tail set —
        // "~", "*", "#", "-", "—" (U+2014) — as the
        // trailing walks' peels do, and a spaced
        // suffix token of the set is taken with the
        // standing separator tokens below.
        let peeled = text[..end]
            .trim_end_matches([
                '.', ',', ';', ':', '!', '?', '"', '\'', '\u{201d}', '\u{2026}', '~', '*', '#',
                '-', '\u{2014}',
            ])
            .len();
        if peeled != end {
            end = peeled;
            continue;
        }
        let core = &text[..end];
        let Some((ws, wc)) = core.char_indices().rev().find(|(_, c)| c.is_whitespace()) else {
            break;
        };
        let token = &core[ws + wc.len_utf8()..];
        if !token.is_empty()
            && token
                .chars()
                .all(|c| matches!(c, '*' | '|' | '~' | '#' | '-' | '\u{2014}'))
        {
            end = ws;
            continue;
        }
        break;
    }
    // The group closes at `group_end`; everything
    // from there on is suffix, preserved as written.
    let group_end = text[..end].trim_end().len();
    let line = &text[..group_end];
    if !matches!(line.chars().next_back(), Some(')' | ']' | '}')) {
        return None;
    }
    // The candidate group, closers aside: the first
    // opener whose remainder — its own pair's marks
    // aside — is one weekday word and nothing else.
    // The remainder's length is checked before its
    // contents are scanned: one weekday word with
    // its wrapping delimiters is a few bytes, and
    // a long remainder is some earlier group, not
    // this line's garnish. Round 34 (R34-E, F3):
    // the candidate openers are all three pairs —
    // a glued bracket or brace garnish ("2026
    // [Monday]" without the space) is judged as
    // its spaced twin and as the paren garnish,
    // each pair's marks judged by the same shape
    // the paren marks have always been judged by.
    let cand = line.trim_end_matches([')', ']', '}']);
    let mut open: Option<usize> = None;
    for (p, ch) in cand.char_indices() {
        if !matches!(ch, '(' | '[' | '{') {
            continue;
        }
        let inner = &cand[p + 1..];
        if inner.len() > 32 || inner.contains(char::is_whitespace) {
            continue;
        }
        // The contents, stated exactly: some opening
        // delimiters of the pair (a doubled garnish
        // wears one extra), the word, some closing
        // delimiters of the pair — a mark of the
        // pair anywhere else is not this shape.
        // Round 36 (R36-C, F9): the opening and
        // closing marks may be ANY pair's — a garnish
        // in a mixed-pair wrapper ("([Monday])") is
        // this shape too, and the candidate is the
        // wrapper's OUTER opener, not its inner one:
        // finding only the inner pair set the space
        // inside the wrapper and broke the token the
        // trailing readers judge, so a mixed wrapper
        // was judged in neither form where its
        // single-pair and doubled same-pair twins
        // are judged; the contents' own marks are
        // read across pairs.
        let trailed = inner.trim_start_matches(['(', '[', '{']);
        let word = trailed.trim_end_matches([')', ']', '}']);
        if word.contains(['(', ')', '[', ']', '{', '}']) || !trailed.starts_with(word) {
            continue;
        }
        if weekday_word(word).is_some() {
            open = Some(p);
            break;
        }
    }
    let open = open?;
    if open == 0 {
        return None;
    }
    let before = text[..open].chars().next_back()?;
    if before.is_whitespace() {
        return None;
    }
    let mut spaced = String::with_capacity(text.len() + 1);
    spaced.push_str(&text[..open]);
    spaced.push(' ');
    spaced.push_str(&text[open..group_end]);
    spaced.push_str(&text[group_end..]);
    Some(spaced)
}

pub(super) fn date_line_inner(
    raw: &Option<String>,
    today: SimpleDate,
) -> (Option<SimpleDate>, Option<WeekdayMismatch>) {
    let Some(text) = raw.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None);
    };
    // Round 33 (R33-B, F2): a glued trailing weekday
    // garnish is judged exactly as its spaced twin
    // (see `spaced_glued_weekday_garnish`): the line
    // the readers below see is the spaced line, and
    // the raw the leading-weekday check sees is that
    // same line. Round 34 (R34-C, F4): the gate's
    // suffix widened there — extra closing marks, a
    // doubled garnish, a trailing separator token,
    // trailing punctuation — and the trailing-word
    // trim set gained the same punctuation, so the
    // spaced forms of those shapes are judged here
    // too.
    let respaced = spaced_glued_weekday_garnish(text);
    let text: &str = respaced.as_deref().unwrap_or(text);
    let effective_raw: Option<String> = match &respaced {
        Some(spaced) => Some(spaced.clone()),
        None => raw.clone(),
    };
    let raw: &Option<String> = &effective_raw;
    if let Some(d) = parse_flexible_date(text, today) {
        return (Some(d), None);
    }
    match sunday_after_outcome(text, today) {
        Some(SundayAfter::Date(d)) => return (Some(d), None),
        Some(SundayAfter::Mismatch(m)) => return (None, Some(m)),
        None => {}
    }
    if let Some(m) = named_weekday_mismatch(raw, today) {
        return (None, Some(m));
    }
    // Round 40 (R40-E, F9): a leading company
    // holding both a governor and a weekday group
    // takes its composed outcome before the readers
    // below inherit the governor-unwrapped line,
    // in which the weekday group stands mid-line
    // behind the bared governor word and the
    // mid-line fallback would serve the date (see
    // `leading_governor_company_judgment`).
    if let Some((_, mismatch, _, _)) = leading_governor_company_judgment(text, today) {
        return (None, mismatch);
    }
    // Round 37 (R37-D, F7): a weekday group standing
    // at the leading or trailing slot with a
    // companion group beside it is judged on its
    // contents, as the lone group is — the group
    // sequence at the slot is walked before the
    // framing reader, whose one-group reach the
    // companions escaped (see
    // `group_sequence_weekday_mismatch`).
    if let Some(m) = group_sequence_weekday_mismatch(text, today) {
        return (None, Some(m));
    }
    // Framing composed (round 7, R7-3/R7-7): a weekday
    // wearing "on" anywhere in the line, and a leading
    // weekday standing with a trailing one, are checked
    // before the plainer readings below get their turn.
    if let Some(outcome) = framed_date_outcome(text, today) {
        return outcome;
    }
    let tokens: Vec<&str> = text.split_whitespace().collect();
    // A weekday written as the line's last word, right
    // after the date (punctuation may wrap it, as in
    // "(Monday)"): take the word out and read what is
    // left. If what is left reads as the date the word
    // names, the line was fine all along; if it reads
    // as a different day, the word was the mistake, and
    // the record says so.
    let last_word = tokens
        .iter()
        .rposition(|t| t.chars().any(|c| c.is_alphanumeric()));
    if let Some(i) = last_word {
        // Round 34 (R34-C, F4 = RT34-5): the trim set
        // extends to ";", ":", "!", "?", and the
        // closing quote — the same marks `clean`
        // trims — so a trailing garnish wearing one
        // is judged here instead of passing as
        // ordinary text in both its forms.
        // Round 35 (R35-A, F1): the set gains the
        // brace pair, so a trailing brace garnish is
        // judged here as its paren and bracket twins
        // are.
        // Round 35 (R35-C, F7): the set gains the
        // apostrophe and the typographic closing
        // quote, the same two marks `clean` gains —
        // a trailing garnish wearing either is judged
        // here instead of passing as ordinary text in
        // both its forms.
        // Round 37 (R37-E, F3): the set gains the
        // rest of the curly family, as `clean` does
        // — a trailing garnish wearing a curly pair
        // ("November 1, 2026 “Monday”") is judged
        // here as its straight-quoted twin is, in
        // both its arms: the disagreeing one refused
        // with the record, the agreeing one read.
        // Round 40 (R40-G, F8): the set gains the
        // ellipsis U+2026, as `clean` does — a
        // trailing garnish wearing one ("November 1,
        // 2026 Monday…", "November 1, 2026
        // (Monday)…") is judged here as one wearing
        // ";" already is, instead of passing as
        // ordinary text and losing its record.
        // Round 41 (R41-E, F6): the trailing garnish
        // is read through the rest of the R40-G
        // sequence-tail set — "~", "*", "#", "-",
        // "—" (U+2014) — shed at the word's tail
        // only, between the two standing trims, so
        // a garnish wearing one ("November 1, 2026
        // Monday~", "November 1, 2026 (Monday)~")
        // is judged here as one wearing the ellipsis
        // already is, while a word merely standing
        // behind one of the marks ("*Monday") keeps
        // its standing glued reading.
        let word = tokens[i]
            .trim_matches([
                '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
                '\u{201c}', '\u{2018}', '\u{2019}', '\u{2026}',
            ])
            .trim_end_matches(['~', '*', '#', '-', '\u{2014}'])
            .trim_matches([
                '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
                '\u{201c}', '\u{2018}', '\u{2019}', '\u{2026}',
            ]);
        // Round 39 (R39-E, F7): the trailing garnish
        // word is read through `cleaned_weekday`, so
        // its interior sentence marks are shed as at
        // the framing and garnish-group readers (the
        // bare path's whole scope since round 40,
        // R40-F) — "(Mon!day)" is judged here as the
        // bare "Mon!day" is.
        if let Some((named, named_number)) = cleaned_weekday(word) {
            // Round 41 (R41-E, F6): a spaced suffix
            // token of the sequence-tail set standing
            // after the garnish word ("November 1,
            // 2026 Monday ~") is the word's dress, as
            // the glued suffix is — when every token
            // after the word is such a token, those
            // tokens leave the rest with the word, so
            // the date the word names is read and the
            // record stands instead of the line dying
            // in silence on a token that is no part
            // of the date.
            let dressed_close = tokens[i + 1..].iter().all(|t| suffix_tail_token(t));
            let rest: Vec<&str> = tokens
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i && !(dressed_close && *j > i))
                .map(|(_, t)| *t)
                .collect();
            if let Some(date) = parse_flexible_date(&rest.join(" "), today) {
                if date.weekday() == named_number {
                    return (Some(date), None);
                }
                return (None, Some(WeekdayMismatch { named, date }));
            }
        }
    }
    // A weekday word standing directly in front of a
    // date that ends the line, with other words before
    // it ("bring a friend Friday, October 25, 2026"),
    // is ordinary text about those other words, not a
    // claim about the date: the date reads, and no
    // mismatch is recorded. (A weekday in the line's
    // first position is the claim case, handled above
    // by `named_weekday_mismatch`.) Defense in depth
    // (round 8, R8-5): a weekday wearing the framing
    // garnish stack was the framing check's to judge,
    // so this fallback never returns the date it
    // stands in front of clean.
    // Round 32 (R32-C, F3): the suffix reader-piece
    // counts for the loop below, computed once. The
    // loop hands the flexible reader the whole
    // suffix after each weekday word, joined and
    // re-tokenized — a line carrying thousands of
    // weekday words (a run of "(next Sunday,
    // cancelled)" groups) paid a whole-line join
    // and parse per word. The reader's own shapes
    // bound what it can ever read from a suffix: at
    // most three reader tokens (a month, a day, and
    // a year, or one numeric token), plus at most
    // one leading weekday token the reader consumes
    // from the same stream — four reader pieces in
    // all, where a piece is a comma-split fragment
    // of a suffix token that the reader does not
    // drop as a nothing. A suffix holding more than
    // four pieces reads as no date, exactly as the
    // reader itself answers for it, so the loop
    // skips only the parses that provably fail; a
    // suffix within the bound is joined and parsed
    // whole, word for word as before.
    //
    // Round 33 (R33-D, F5): the premise the count
    // rests on, stated where it is used. The count
    // equals the reader's own token count exactly
    // when `nothing_token` here is the predicate
    // `parse_flexible_date` tokenizes by — and it
    // is: both name ob-model's one `nothing_token`
    // (see `nothing_token` in dates.rs, which
    // delegates to it), so the agreement holds by
    // construction. If a local copy ever reappears
    // on either side and the copies drift, this
    // skip silently becomes a moved reading: a
    // fragment this count keeps but the reader
    // would drop inflates the count and skips a
    // suffix the reader could have read; a fragment
    // this count drops but the reader keeps deflates
    // it and the bound's proof collapses the other
    // way. The fragment family both predicates
    // judge is pinned in
    // `r33d_nothing_token_is_the_flexible_readers_predicate`.
    let suffix_pieces: Vec<usize> = {
        let mut counts: Vec<usize> = vec![0; tokens.len() + 1];
        for i in (0..tokens.len()).rev() {
            let pieces = tokens[i]
                .split(',')
                .filter(|f| !f.is_empty() && !nothing_token(f))
                .count();
            counts[i] = counts[i + 1] + pieces;
        }
        counts
    };
    for i in 1..tokens.len() {
        // Round 35 (R35-A, F1): the trim set gains
        // the brace pair, as at the scans above.
        // Round 38 (R38-F, F7 = review R38-R2): the
        // set gains the quote family — the straight
        // pair, the apostrophe, and the curly family,
        // the same marks `clean` trims — so a weekday
        // word standing mid-line in quote dress
        // ("notes \"Monday\" November 1, 2026") is the
        // word it names here, as its bare twin is: a
        // quote-wrapped word is a word in dress, and
        // at this slot the bare word is ordinary text
        // the date reads past. Before this, the trim
        // stopped at the quotes, no reader in the
        // line ever saw the weekday, and the line
        // read nothing in any quote form while its
        // bare and grouped twins served the date.
        let word = tokens[i].trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', '"', '\'', '\u{201c}', '\u{201d}', '\u{2018}',
            '\u{2019}',
        ]);
        // Round 29 (R29-A, F3 = red team RT29-T1):
        // the weekday word of a bare phrase unit is
        // judged on the unglued token vector, as
        // the unit's membership is — a unit whose
        // weekday word wears the glue ("see you
        // *sunday November 1, 2026") stands in front
        // of the date it trails exactly as its
        // clean twin's weekday does, so the date
        // reads. A glued weekday ending no unit
        // keeps its standing reading (see
        // `bare_unit_weekday_at`).
        if weekday_word(word).is_none() && !bare_unit_weekday_at(&tokens, i) {
            continue;
        }
        if weekday_is_framed(&tokens, i) {
            continue;
        }
        if suffix_pieces[i + 1] > 4 {
            continue;
        }
        if let Some(date) = parse_flexible_date(&tokens[i + 1..].join(" "), today) {
            return (Some(date), None);
        }
    }
    (None, None)
}

/// What one date line reads, all records together
/// (round 9, R9-C): the date, the weekday mistake when
/// one is named, the parenthesized garnish's
/// disagreement when one stands, and a second absolute
/// date named later in the line when one stands.
pub(super) struct DateLineReading {
    pub(super) date: Option<SimpleDate>,
    pub(super) weekday_mismatch: Option<WeekdayMismatch>,
    pub(super) parenthesized_mismatch: Option<ParenthesizedDateMismatch>,
    pub(super) second_dates: Vec<SimpleDate>,
}

/// The group delimiters' closer for an opener
/// (round 34, R34-E, F3): since this round the
/// group machinery treats all three pairs alike —
/// a bracketed or braced group is judged as its
/// paren twin. Every delimiter is one byte, so the
/// span arithmetic (+1 / -1 at the edges) is the
/// same for each pair.
pub(super) fn group_closer(opener: u8) -> u8 {
    match opener {
        b'(' => b')',
        b'[' => b']',
        _ => b'}',
    }
}

/// Whether a byte opens a group, in any of the
/// three pairs (round 34, R34-E).
pub(super) fn is_group_opener(b: u8) -> bool {
    matches!(b, b'(' | b'[' | b'{')
}

/// Whether a byte closes a group, in any of the
/// three pairs (round 34, R34-E).
pub(super) fn is_group_closer(b: u8) -> bool {
    matches!(b, b')' | b']' | b'}')
}

/// The parenthesized groups standing in `text`, as
/// (start, end) byte spans, end past the closing
/// delimiter: flat groups only, in written order.
/// (The name is the standing one; since round 34
/// the groups may wear brackets or braces as well
/// as parentheses, each pair judged alike — a flat
/// group is one whose inner text holds no opener
/// of its OWN pair, as the paren rule has always
/// been. An opener with no closer of its pair is
/// passed over, so an unclosed bracket standing
/// before a closed paren group hides nothing.)
///
/// Round 35 (R35-E, F5): the pass-over is held in a
/// per-pair closer-absence memory. A forward search
/// that finds no closer of a pair proves no closer
/// of that pair stands anywhere in the remaining
/// text, so every later opener of that pair is
/// passed over without searching again. The walk's
/// other searches never overlap — a found closer
/// moves `from` past itself — so a run of unclosed
/// openers, which used to rescan the whole
/// remaining line per opener and cost quadratic
/// time, now costs one failed search per pair and
/// one step per opener. The spans found are the
/// standing ones, step for step: the memory only
/// skips searches already known to fail.
pub(super) fn parenthesized_spans(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut from = 0;
    let mut closer_absent = [false; 3];
    while from < bytes.len() {
        let Some(rel_open) = bytes[from..].iter().position(|b| is_group_opener(*b)) else {
            break;
        };
        let open = from + rel_open;
        let opener = bytes[open];
        let pair = match opener {
            b'(' => 0,
            b'[' => 1,
            _ => 2,
        };
        if closer_absent[pair] {
            from = open + 1;
            continue;
        }
        let closer = group_closer(opener);
        let Some(rel_close) = bytes[open + 1..].iter().position(|b| *b == closer) else {
            closer_absent[pair] = true;
            from = open + 1;
            continue;
        };
        let close = open + 1 + rel_close;
        let inner = &text[open + 1..close];
        if !inner.contains(opener as char) {
            out.push((open, close + 1));
        }
        from = close + 1;
    }
    out
}

/// The top-level parenthesized spans of `text`, as
/// (start, end) byte spans, end past the closing
/// delimiter, in written order (round 21, R21-1):
/// found by balancing, so a nested group is one span
/// ("(note (November 1))"), and a group standing
/// anywhere in the line — leading, between plain
/// segments, trailing — is seen. An unbalanced
/// opening delimiter forms no span. Round 34
/// (R34-E, F3): the balance is a stack over all
/// three delimiter pairs — a closer closes the
/// opener it matches, and a closer matching nothing
/// on the stack closes nothing — so a bracketed or
/// braced group is a span exactly as its paren twin
/// is. On parenthesis-only text the stack is the
/// depth count it replaces, step for step: a stray
/// ")" with nothing open is ignored, as it was.
pub(super) fn balanced_spans(text: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let bytes = text.as_bytes();
    let mut stack: Vec<(usize, u8)> = Vec::new();
    for (i, b) in bytes.iter().enumerate() {
        if is_group_opener(*b) {
            stack.push((i, group_closer(*b)));
            continue;
        }
        let matched = is_group_closer(*b) && stack.last().map(|&(_, want)| want) == Some(*b);
        if matched {
            let (open, _) = stack.pop().expect("a matched opener stands");
            if stack.is_empty() {
                out.push((open, i + 1));
            }
        }
    }
    out
}

/// `text` with a sentence mark glued to a bare
/// month word or day number removed, when any stand
/// (round 37, R37-F, F9 = red team RT37-7): the
/// grouped readers shed such a mark at the word's
/// edge — "(November! 8, 2026)" reads November 8,
/// because the group cleaning trims "!", "?", ":",
/// and ";" from a token's edges — but the bare
/// line's own reader (`parse_flexible_date`) trims
/// only a trailing ".", so "November! 1, 2026",
/// "November? 1, 2026", "November: 1, 2026", and
/// "November 1!, 2026" read nothing on the bare
/// path, in silence. The adjudicated reading is
/// that a sentence mark standing against a month
/// word or a day number is the same garnish on the
/// bare path the grouped readers already shed, so
/// the mark is removed where it stands glued in
/// exactly the two witnessed slots: at a month
/// word's tail, with the day number standing after
/// it (whitespace between), and at a day number's
/// tail, with the date's comma standing after it.
/// A mark standing anywhere else — against another
/// word, before a word — keeps its bytes and its
/// standing effect, exactly as a stray group mark
/// outside `stray_mark_spaced`'s shapes does. The
/// removal drops the mark byte only; every other
/// byte stands as written.
///
/// Round 38 (R38-F, F8 = review R38-R3, F9 = red
/// team RT38-5, F10 = red team RT38-7): the shed
/// stopped one slot short in three places, and one
/// sentence above recorded a year's tail as keeping
/// its standing effect — a doc sentence recording
/// an effect is not an adjudication of it, and the
/// grouped twins read through the same marks at the
/// same slots, so the bare path takes their reading
/// and that sentence is corrected here. The day
/// slot gains the period ("November 1., 2026" read
/// nothing; "(November 1.), 2026" reads). A year's
/// tail joins the shed: a run of sentence marks and
/// closing quotes standing at the line's end
/// against a year (two or four digits, the date's
/// comma before it, outside any group) is dropped
/// ("November 1, 2026!", "November 1, 2026..",
/// "November 1, 26!" all read nothing bare, and
/// read grouped). And a mark standing INSIDE a
/// bare date word is dropped where the word it
/// splits is a date word once the mark is gone —
/// a month or weekday name ("Novem!ber 8, 2026",
/// "Sun!day after November 1, 2026"), or a digit
/// run of two or four ("November 1, 20!26",
/// "November 1!2, 2026") — outside any group, so
/// the grouped readers' own readings of such words
/// are untouched. A mark inside any other word
/// keeps its bytes: it splits no date word, and
/// shedding it could only mint one (a governor's
/// stem is not this pass's to complete).
///
/// Round 39 (R39-F, F8 = review R39-R8 + red team
/// RT39-6): the shed stopped one mark short at
/// four further places, each witnessed silent
/// against a reading twin. The month and day
/// slots judge a mark RUN as they judged one
/// mark ("November!! 1, 2026", "November 1!!,
/// 2026"), and the day slot's period with them
/// ("November 1.., 2026"). The year-tail arm
/// admits a year standing after a day number
/// with whitespace between, comma or none
/// ("November 1 2026!"). The interior arm sheds
/// a run that ends in a mark as well as one that
/// holds a mark inside, under the same
/// vocabulary anchor ("Sun!day! after November
/// 1, 2026"). And a mark standing at a day
/// token's head, a month word before it and the
/// date's comma after the number, is shed as a
/// mark at its tail is ("November !1, 2026").
pub(super) fn bare_date_mark_shed(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut drop: Vec<usize> = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        if !matches!(b, b'!' | b'?' | b':' | b';' | b'.') {
            continue;
        }
        // Round 39 (R39-F, F8 = review R39-R8 +
        // red team RT39-6): the slots below judge
        // the mark RUN the mark stands in, not the
        // single mark — a run at the month or day
        // slot ("November!! 1, 2026", "November
        // 1!!, 2026", "November 1.., 2026") shed
        // nothing, because at the run's first mark
        // the next byte is its fellow and at its
        // last the scan behind finds no word or
        // number, where the grouped twins read
        // through the run as they read through one
        // mark. The run is the maximal run of
        // sentence marks holding this mark.
        let mut run_start = i;
        while run_start > 0 && matches!(bytes[run_start - 1], b'!' | b'?' | b':' | b';' | b'.') {
            run_start -= 1;
        }
        let mut run_end = i + 1;
        while run_end < bytes.len() && matches!(bytes[run_end], b'!' | b'?' | b':' | b';' | b'.') {
            run_end += 1;
        }
        // The month slot: the letter run ending at
        // the mark run is a month word, and a day
        // digit stands after the run.
        let mut start = run_start;
        while start > 0 && bytes[start - 1].is_ascii_alphabetic() {
            start -= 1;
        }
        if start < run_start && word_at_is_month(text, start) {
            let mut j = run_end;
            while j < bytes.len() && matches!(bytes[j], b' ' | b'\t') {
                j += 1;
            }
            if j < bytes.len() && bytes[j].is_ascii_digit() {
                drop.extend(run_start..run_end);
                continue;
            }
        }
        // The day slot: a day number (one or two
        // digits, 1..=31) ends at the mark run, and
        // the date's comma stands immediately after
        // the run.
        let mut start = run_start;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        let digits = &text[start..run_start];
        if !digits.is_empty()
            && digits.len() <= 2
            && digits
                .parse::<u32>()
                .is_ok_and(|day| (1..=31).contains(&day))
            && bytes.get(run_end) == Some(&b',')
        {
            drop.extend(run_start..run_end);
            continue;
        }
        // The day token's head (round 39, R39-F,
        // F8 = red team RT39-6): a mark run standing
        // against the head of a day number, with a
        // month word before it and the date's comma
        // after the number, is the same garnish at
        // the token's other edge — "November !1,
        // 2026" read nothing, where the grouped
        // "(November !1, 2026)" reads, its cleaning
        // trimming the token's head as its tail.
        let mut dend = run_end;
        while dend < bytes.len() && bytes[dend].is_ascii_digit() {
            dend += 1;
        }
        let head_digits = &text[run_end..dend];
        if !head_digits.is_empty()
            && head_digits.len() <= 2
            && head_digits
                .parse::<u32>()
                .is_ok_and(|day| (1..=31).contains(&day))
            && bytes.get(dend) == Some(&b',')
            && word_before_is_month(text, run_start)
        {
            drop.extend(run_start..run_end);
        }
    }
    // Round 38 (R38-F): the year tail and the
    // interior marks (see the head note). Both arms
    // judge only marks standing outside every
    // balanced group: inside a group the grouped
    // readers' own cleaning already sheds a year's
    // tail mark, and an interior mark there is the
    // group readers' standing silence, not this
    // pass's to move.
    let spans = balanced_spans(text);
    // Round 39 (R39-G, F9/F11): the spans are the
    // top-level balanced spans — sorted by start
    // and disjoint — so the span that could hold
    // `pos` is the last one starting before it,
    // found by binary search. The linear scan this
    // replaces answered the same question per mark,
    // and a line carrying thousands of groups and
    // thousands of marks (the semicolon-group line)
    // paid groups × marks for it: quadratic time.
    // The predicate is the standing one, word for
    // word: `pos` strictly inside that one span.
    let inside_group = |pos: usize| {
        let at = spans.partition_point(|(s, _)| *s < pos);
        at > 0 && pos < spans[at - 1].1
    };
    // The year slot: peel the line's end — sentence
    // marks and closing quotes, whitespace between
    // and around them — and judge what stands
    // before the peel: a year of two or four digits
    // with the date's comma before it.
    {
        let mut end = text.len();
        let mut tail_marks: Vec<usize> = Vec::new();
        while let Some((ci, c)) = text[..end].char_indices().next_back() {
            if c.is_whitespace() {
                end = ci;
                continue;
            }
            if matches!(
                c,
                '!' | '?' | ':' | ';' | '.' | '\u{2026}' | '"' | '\'' | '\u{201d}' | '\u{2019}'
            ) {
                tail_marks.push(ci);
                end = ci;
                continue;
            }
            break;
        }
        if !tail_marks.is_empty() {
            let mut start = end;
            while start > 0 && bytes[start - 1].is_ascii_digit() {
                start -= 1;
            }
            let digits = &text[start..end];
            if (digits.len() == 2 || digits.len() == 4) && !inside_group(start) {
                let mut before = start;
                while before > 0 && matches!(bytes[before - 1], b' ' | b'\t') {
                    before -= 1;
                }
                // Round 39 (R39-F, F8 = review
                // R39-R8): the comma the arm has
                // always required is the date's own
                // comma, which the comma-less form
                // lawfully lacks — "November 1
                // 2026!" read nothing where the
                // clean comma-less form serves and
                // the grouped twin reads. A year
                // standing after a day number with
                // whitespace between, comma or
                // none, carries the same tail.
                let mut dstart = before;
                while dstart > 0 && bytes[dstart - 1].is_ascii_digit() {
                    dstart -= 1;
                }
                let day_digits = &text[dstart..before];
                let after_day = !day_digits.is_empty()
                    && day_digits.len() <= 2
                    && day_digits
                        .parse::<u32>()
                        .is_ok_and(|day| (1..=31).contains(&day));
                if (before > 0 && bytes[before - 1] == b',') || after_day {
                    for m in tail_marks {
                        let width = text[m..]
                            .chars()
                            .next()
                            .expect("a mark stands there")
                            .len_utf8();
                        drop.extend(m..m + width);
                    }
                }
            }
        }
    }
    // The interior slots: a sentence mark standing
    // between two letters of a month or weekday
    // name, or between two digits of a two- or
    // four-digit run, once the run's marks are gone.
    for (i, &b) in bytes.iter().enumerate() {
        if !matches!(b, b'!' | b'?' | b':' | b';') || inside_group(i) {
            continue;
        }
        // The letter run holding the mark.
        let mut start = i;
        while start > 0
            && (bytes[start - 1].is_ascii_alphabetic()
                || matches!(bytes[start - 1], b'!' | b'?' | b':' | b';'))
        {
            start -= 1;
        }
        let mut end = i + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphabetic() || matches!(bytes[end], b'!' | b'?' | b':' | b';'))
        {
            end += 1;
        }
        // Round 39 (R39-F, F8 = red team RT39-6):
        // the run may END in a mark as well as hold
        // one inside — "Sun!day! after November 1,
        // 2026" shed nothing, because the run's
        // last byte is the second mark, not a
        // letter, where the single-mark form held
        // in R38-F's family. A run holding at least
        // one mark between two letters, whose
        // letters joined are a month or weekday
        // name under the same vocabulary anchor,
        // sheds every mark it holds; a word wearing
        // only a trailing mark ("Sunday!") is not
        // this arm's — it holds no interior mark,
        // and its standing readers judge it as
        // they always have.
        let has_interior = (start + 1..end.saturating_sub(1)).any(|p| {
            matches!(bytes[p], b'!' | b'?' | b':' | b';')
                && bytes[p - 1].is_ascii_alphabetic()
                && bytes[p + 1].is_ascii_alphabetic()
        });
        if end - start > 1
            && bytes[start].is_ascii_alphabetic()
            && (bytes[end - 1].is_ascii_alphabetic() || has_interior)
        {
            // Round 40 (R40-F, F7): the joined word
            // is `interior_mark_shed`'s — the one
            // shed both interior arms state, so this
            // arm and `cleaned_weekday` shed the
            // same marks by construction.
            let joined: String = interior_mark_shed(&text[start..end]);
            if word_at_is_month(&joined, 0) || weekday_word(&joined).is_some() {
                drop.extend(
                    (start..end).filter(|p| matches!(bytes[*p], b'!' | b'?' | b':' | b';')),
                );
                continue;
            }
        }
        // The digit run holding the mark.
        let mut start = i;
        while start > 0
            && (bytes[start - 1].is_ascii_digit()
                || matches!(bytes[start - 1], b'!' | b'?' | b':' | b';'))
        {
            start -= 1;
        }
        let mut end = i + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_digit() || matches!(bytes[end], b'!' | b'?' | b':' | b';'))
        {
            end += 1;
        }
        if end - start > 1 && bytes[start].is_ascii_digit() && bytes[end - 1].is_ascii_digit() {
            let digits = text[start..end]
                .chars()
                .filter(|c| !matches!(c, '!' | '?' | ':' | ';'))
                .count();
            if digits == 2 || digits == 4 {
                drop.extend(
                    (start..end).filter(|p| matches!(bytes[*p], b'!' | b'?' | b':' | b';')),
                );
            }
        }
    }
    if drop.is_empty() {
        return None;
    }
    let drop_set: std::collections::BTreeSet<usize> = drop.into_iter().collect();
    let shed_bytes: Vec<u8> = bytes
        .iter()
        .enumerate()
        .filter(|(p, _)| !drop_set.contains(p))
        .map(|(_, b)| *b)
        .collect();
    Some(String::from_utf8(shed_bytes).expect("only whole marks were dropped"))
}

/// `text` with the quote dress of a fully
/// quote-wrapped date word removed, when one
/// stands at its own slot (round 39, R39-F, F8 =
/// red team RT39-7): past R38-F's quoted-weekday
/// fix, the quoted DATE words at their own slots
/// still silenced the line — "\"November\" 1,
/// 2026" and "November \"1\", 2026" read nothing
/// on either route, where the clean twins read.
/// A fully quote-wrapped word is transparent at
/// the slot where its bare twin is judged, as the
/// quoted weekday word became at R38-F: the month
/// word standing as the line's first token or at
/// a Sunday-after phrase's base slot, in
/// any quote form (the straight pair, the
/// apostrophe pair, the curly family, mixed), and
/// the day number standing after a month word,
/// bare or itself quoted, with the date's comma
/// after its closing quote. The removal drops
/// the two quote characters only; every other
/// byte stands as written. The anchors are the
/// standing vocabulary and the day range: a
/// quoted word that is not a month name keeps its
/// quotes (a quoted governor's dress is not this
/// pass's to strip — it is judged by the readers
/// that judge it bare), and a quoted number after
/// any other word keeps its quotes too.
pub(super) fn bare_date_quote_shed(text: &str) -> Option<String> {
    // The line's tokens, as byte ranges.
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut at: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(start) = at.take() {
                tokens.push((start, i));
            }
        } else if at.is_none() {
            at = Some(i);
        }
    }
    if let Some(start) = at {
        tokens.push((start, text.len()));
    }
    // A fully quote-wrapped token's inner word and
    // the byte widths of its two quote characters,
    // when it is wrapped: an opening quote of the
    // family at its head and a closing quote at
    // its tail, distinct characters.
    fn unquoted(token: &str) -> Option<(&str, usize, usize)> {
        let first = token.chars().next()?;
        let last = token.chars().last()?;
        if !matches!(first, '"' | '\'' | '\u{201c}' | '\u{2018}')
            || !matches!(last, '"' | '\'' | '\u{201d}' | '\u{2019}')
        {
            return None;
        }
        let open = first.len_utf8();
        let close = last.len_utf8();
        if token.len() <= open + close {
            return None;
        }
        Some((&token[open..token.len() - close], open, close))
    }
    let mut drop: Vec<(usize, usize)> = Vec::new();
    // The month slot: the line's first token, or
    // the base slot of a Sunday-after phrase (the
    // two places `month_group_reading` splices a
    // grouped month) — a quoted month standing
    // after any other words is the interior-slot
    // parity the round-39 review record states
    // (quoted == bare silence there), not this
    // pass's to move.
    let cleaned_prefix: Vec<String> = tokens
        .iter()
        .take(3)
        .map(|&(s, e)| group_token(&text[s..e]))
        .collect();
    let month_idx = if cleaned_prefix.first().map(String::as_str) == Some("sunday")
        && cleaned_prefix.get(1).map(String::as_str) == Some("after")
    {
        2
    } else if cleaned_prefix.first().map(String::as_str) == Some("the")
        && cleaned_prefix.get(1).map(String::as_str) == Some("sunday")
        && cleaned_prefix.get(2).map(String::as_str) == Some("after")
    {
        3
    } else {
        0
    };
    let &(mstart, mend) = tokens.get(month_idx)?;
    let month_token = &text[mstart..mend];
    let month_bare =
        month_token.chars().all(|c| c.is_ascii_alphabetic()) && word_at_is_month(month_token, 0);
    let month_quoted = match unquoted(month_token) {
        Some((inner, open, close))
            if inner.chars().all(|c| c.is_ascii_alphabetic()) && word_at_is_month(inner, 0) =>
        {
            drop.push((mstart, mstart + open));
            drop.push((mend - close, mend));
            true
        }
        _ => false,
    };
    if month_bare || month_quoted {
        if let Some(&(dstart, dend)) = tokens.get(month_idx + 1) {
            let day_token = &text[dstart..dend];
            let core = day_token.strip_suffix(',').unwrap_or(day_token);
            if let Some((inner, open, close)) = unquoted(core) {
                let digits_ok = !inner.is_empty()
                    && inner.len() <= 2
                    && inner.chars().all(|c| c.is_ascii_digit())
                    && inner
                        .parse::<u32>()
                        .is_ok_and(|day| (1..=31).contains(&day));
                if digits_ok {
                    drop.push((dstart, dstart + open));
                    drop.push((dstart + core.len() - close, dstart + core.len()));
                }
            }
        }
    }
    if drop.is_empty() {
        return None;
    }
    drop.sort_unstable();
    let shed_text: String = {
        let mut out = String::with_capacity(text.len());
        let mut at = 0usize;
        for (s, e) in &drop {
            out.push_str(&text[at..*s]);
            at = *e;
        }
        out.push_str(&text[at..]);
        out
    };
    Some(shed_text)
}

/// `text` with the stray group marks the shared
/// stack walk does not close set to spaces, in
/// the shapes scoped below, when any stand
/// (round 35, R35-F, F6, as adjudicated
/// 2026-10-21): an opener the walk never closes is
/// a stray mark, inert at the plain readers — the
/// treatment an unmatched "[" or "{" already
/// received, because `parse_date_prefix`'s edge
/// trim stops only at the two parentheses. A date
/// token wearing an unclosed "(" was therefore not
/// a signal while its bracket and brace twins read,
/// and that one asymmetry was the whole live
/// mechanism of the malformed family: the group
/// machinery already forms no group for these
/// shapes (the walk closes nothing, the reverse
/// walk finds no trailing group, the flat search
/// passes the openers over). Setting the stray
/// marks to spaces hands the composed readers the
/// text the ruling names — the same words with
/// their unmatched marks removed — while a
/// walk-closed group's marks keep their bytes, so
/// its edge keeps the structural effect the paren
/// rule's designed reliance stands on (the
/// raw-text scans still meet a parse failure at a
/// real group edge, and a closed group's inner
/// date is never a plain signal). The spacing is
/// scoped to the adjudicated change surface, in
/// two shapes. The first is the paren pair itself:
/// every unmatched "(" is spaced, and a stray ")"
/// is spaced where it stands glued to the head of
/// a word (")November"), wearing the word exactly
/// as an unmatched opener does; a stray closer at
/// a word's tail or standing alone keeps its
/// standing effect untouched, as the round-35
/// battery's stray-close cells hold. The second
/// shape is an unmatched "[" or "{" standing
/// glued to the head of a walk-closed group
/// ("[(November 8, 2026), cancelled)"): there the
/// mark wears the real group's edge exactly as
/// "(November" wears a date token's, and it is
/// spaced with the group's own marks left
/// standing. An unmatched bracket that opens over
/// WORDS keeps its bytes: the bracket span
/// machinery has its own standing reading for
/// unclosed spans (the R26-C restatement mask
/// binds an unclosed bracket span as a closed one,
/// re-affirmed at R34-E), and that reading is not
/// this ruling's to move. Round 36 (R36-F, F8 =
/// review R36-8) extends the surface in two places
/// where the paren pair already judged: the stray
/// closer glued to a word's head is spaced for all
/// three pairs, not the paren pair alone, and an
/// unmatched "[" or "{" standing at the head of
/// the line is spaced — the mask's binding runs
/// beside a governed sibling, and a mark leading
/// the line has none; a mark standing after other
/// content keeps its bytes, so the R26-C shapes
/// hold. Round 37 (R37-F, F10 = red team RT37-8)
/// extends the closer surface in the two places
/// the wrapped twins already judged: the head rule
/// looks past a run of fellow closers standing
/// against the same word ("))November" is the
/// single stray twice over — the first of the pair
/// stands against a closer, not the word, and read
/// nothing where the single stray reads), and a
/// stray closer standing at the line's END is
/// spaced (the trailing stray of "November 1,
/// 2026)", whose wrapped twin "(November 1,
/// 2026))" reads through its extra closer). The
/// round-35 tail hold survives only mid-line: a
/// stray closer at a word's tail with more line
/// after it ("November 1], 2026") keeps its bytes.
/// Round 38 (R38-F, F8 = review R38-R3) extends the
/// trailing arm once more, in the place the wrapped
/// twins already judged: a stray standing in the
/// line's trailing garnish run — closers and
/// sentence marks to the line's end — is spaced
/// with the run's last stray (see
/// `stray_closer_is_spaced`).
/// The walk is
/// `balanced_spans`' own, statement for statement,
/// except that what it records is the marks
/// themselves.
pub(super) fn stray_mark_spaced(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    // Round 39 (R39-G, F9): the trailing-garnish
    // suffix table the stray test below reads —
    // `tail_garnish[p]` is whether every character
    // of `text[p..]` is whitespace or a member of
    // the trailing run set (closers, sentence marks,
    // the closing quotes), the question the test's
    // first two arms ask of the remainder after a
    // stray. Computed once, in one reverse pass:
    // the per-stray remainder scans this replaces
    // cost a whole remainder per stray closer, and
    // the unmatched-opener filter further down made
    // the same shape of rescan per opener (a linear
    // `contains` over the walk's own lists inside a
    // per-byte `all`), so a line of thousands of
    // unclosed mixed openers ("([({([({…") paid
    // quadratic-to-cubic time for this one pass.
    // Every table below states the standing
    // predicate, position for position; no judgment
    // changes.
    let mut tail_garnish: Vec<bool> = vec![true; bytes.len() + 1];
    {
        let mut good = true;
        for (idx, c) in text.char_indices().rev() {
            good = (c.is_whitespace()
                || matches!(
                    c,
                    ')' | ']'
                        | '}'
                        | '.'
                        | '!'
                        | '?'
                        | ':'
                        | ';'
                        | '\u{2026}'
                        | '"'
                        | '\''
                        | '\u{201c}'
                        | '\u{201d}'
                        | '\u{2018}'
                        | '\u{2019}'
                ))
                && good;
            tail_garnish[idx] = good;
        }
    }
    let mut stack: Vec<usize> = Vec::new();
    let mut closed_openers: Vec<usize> = Vec::new();
    let mut stray: Vec<usize> = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        if is_group_opener(b) {
            stack.push(i);
            continue;
        }
        if is_group_closer(b) {
            if stack
                .last()
                .is_some_and(|&open| group_closer(bytes[open]) == b)
            {
                closed_openers.push(stack.pop().expect("a matched opener stands"));
            } else if stray_closer_is_spaced(text, i, &tail_garnish) {
                // Round 36 (R36-F, F8 = review R36-8):
                // the stray closer glued to a word's
                // head is spaced for all three pairs,
                // as the paren closer has been since
                // round 35 — "]November" and "}November"
                // wear their word exactly as
                // ")November" does.
                // Round 37 (R37-F, F10): the spacing
                // test itself (`stray_closer_is_spaced`)
                // carries the two extensions — the
                // head rule looking past a run of
                // fellow closers, and the stray closer
                // standing at the line's end.
                stray.push(i);
            }
        }
    }
    // Round 39 (R39-G, F9): the filter's membership
    // questions, read off tables built in one pass
    // each (see the head note): `closed_set` is
    // `closed_openers` as a per-byte table,
    // `unmatched_set` is the surviving stack as one,
    // and `prefix_good[p]` is the head arm's `all`
    // over `bytes[..p]` — every byte before `p` is
    // ASCII whitespace or an unmatched opener —
    // accumulated as a prefix conjunction, which is
    // the same predicate: the arm's scan is a pure
    // conjunction over a prefix.
    let closed_set: Vec<bool> = {
        let mut set = vec![false; bytes.len()];
        for &open in &closed_openers {
            set[open] = true;
        }
        set
    };
    let unmatched_set: Vec<bool> = {
        let mut set = vec![false; bytes.len()];
        for &open in &stack {
            set[open] = true;
        }
        set
    };
    let prefix_good: Vec<bool> = {
        let mut good = vec![true; bytes.len() + 1];
        for j in 0..bytes.len() {
            good[j + 1] = good[j]
                && (bytes[j].is_ascii_whitespace()
                    || (is_group_opener(bytes[j]) && unmatched_set[j]));
        }
        good
    };
    stray.extend(stack.iter().copied().filter(|&open| {
        bytes[open] == b'('
            || (open + 1 < bytes.len()
                && is_group_opener(bytes[open + 1])
                && closed_set[open + 1])
            // Round 36 (R36-F, F8 = review R36-8): an
            // unmatched "[" or "{" standing at the
            // head of the line — nothing before it but
            // whitespace and other unmatched openers —
            // is a stray mark too, spaced as its paren
            // twin is: "[November 1, 2026" is the bare
            // line with a typo'd mark, and no governed
            // sibling stands before it for the R26-C
            // restatement mask to bind beside. A mark
            // standing after other content keeps its
            // bytes: that is the R26-C surface, and it
            // is not this ruling's to move.
            || (bytes[open] != b'(' && prefix_good[open])
    }));
    if stray.is_empty() {
        return None;
    }
    let mut spaced = bytes.to_vec();
    for p in stray {
        spaced[p] = b' ';
    }
    Some(String::from_utf8(spaced).expect("only ASCII delimiter bytes were set to spaces"))
}

/// Whether the stray (walk-unmatched) closer at
/// byte `i` of `text` is one `stray_mark_spaced`
/// spaces (round 37, R37-F, F10 = red team RT37-8,
/// stating the head rule round 35 set and round 36
/// widened to all three pairs, with this round's
/// two extensions): the closer stands glued to a
/// word's head — the next word character after it,
/// looking past any run of fellow closers standing
/// against the same word, so each closer of
/// "))November" is judged as the single stray is —
/// or the closer stands at the line's end, with
/// nothing after it but whitespace (the trailing
/// stray of "November 1, 2026)"). A stray closer
/// standing anywhere else — at a word's tail
/// mid-line, or before punctuation or a space —
/// is not spaced here and keeps its standing
/// effect.
///
/// Round 38 (R38-F, F8 = review R38-R3): the
/// trailing arm stopped one shape short — it
/// spaced only a stray with nothing but whitespace
/// after it, so in a run only the last stray was
/// spaced and the rest killed the date
/// ("November 1, 2026))", "November 1, 2026)]"),
/// and a stray wearing a sentence mark at the
/// line's end was spaced in neither form
/// ("November 1, 2026).", "November 1, 2026]."),
/// where the wrapped twins read through the run
/// and the mark. The trailing arm therefore also
/// spaces a stray whose whole remainder is the
/// line's trailing garnish run — fellow closers,
/// sentence marks, and whitespace, nothing else —
/// each stray of the run judged as the single
/// trailing stray is. The mid-line hold stands:
/// a stray with a word, a comma, or a date after
/// its run is not spaced here.
///
/// Round 39 (R39-F, F8 = review R39-R8): the run
/// set gains the closing-quote family — the
/// straight pair, the apostrophe, and the curly
/// opening and closing quotes, the same family
/// the year-tail peel sheds — so a stray standing
/// before a closing quote is spaced as a stray
/// standing before a sentence mark is
/// ("November 1, 2026)”", "November 1, 2026)'"
/// read nothing, where "November 1, 2026)!” —
/// the quote in the year tail, no stray — and
/// the wrapped twins read).
fn stray_closer_is_spaced(text: &str, i: usize, tail_garnish: &[bool]) -> bool {
    let bytes = text.as_bytes();
    // Round 39 (R39-G, F9): the first two arms —
    // the remainder is all whitespace, or every
    // character of it stands in the trailing run
    // set — are the one suffix question
    // `tail_garnish[i + 1]` answers (the caller's
    // table, built once per line): an all-whitespace
    // remainder satisfies the run set's predicate
    // character for character, and an empty
    // remainder satisfies both, so the table states
    // the two arms together, exactly.
    if tail_garnish[i + 1] {
        return true;
    }
    let mut j = i + 1;
    while j < bytes.len() && is_group_closer(bytes[j]) {
        j += 1;
    }
    j < bytes.len()
        && text[j..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric())
}

/// The first top-level parenthesized span of
/// `text`, if one closes (round 32, R32-C, F3):
/// `balanced_spans`' first entry, read by the same
/// balancing walk, which stops where that span
/// closes instead of walking the whole text. The
/// readers that strip or judge one leading group
/// at a time — the relative-shape loop, the
/// leading-group disagreement test — used to take
/// the first entry of the full span list, so a
/// line holding thousands of leading groups paid
/// for the whole remainder once per group. The
/// walk below is `balanced_spans`' own, statement
/// for statement, up to the first span it records:
/// since round 34 (R34-E) the walk is the stack
/// over all three delimiter pairs — an opener
/// pushes its expected closer, a closer that
/// matches the stack's top pops it, and the pop
/// that empties the stack records the span.
pub(super) fn first_balanced_span(text: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut stack: Vec<(usize, u8)> = Vec::new();
    for (i, b) in bytes.iter().enumerate() {
        if is_group_opener(*b) {
            stack.push((i, group_closer(*b)));
            continue;
        }
        let matched = is_group_closer(*b) && stack.last().map(|&(_, want)| want) == Some(*b);
        if matched {
            let (open, _) = stack.pop().expect("a matched opener stands");
            if stack.is_empty() {
                return Some((open, i + 1));
            }
        }
    }
    None
}

/// `text` without its leading group, when it begins
/// with one (round 21, R21-1): the words after the
/// group. Found by balancing, like
/// `split_last_trailing_group` at the other end.
/// (Since round 34 the leading group may wear
/// brackets or braces as well as parentheses.)
pub(super) fn strip_leading_parenthesized(text: &str) -> Option<&str> {
    let text = text.trim_start();
    if !matches!(text.chars().next(), Some('(' | '[' | '{')) {
        return None;
    }
    let (start, end) = first_balanced_span(text)?;
    if start != 0 {
        return None;
    }
    Some(text[end..].trim_start())
}

/// A parenthesized group standing between a month-day
/// and its year does not poison the plain date (round
/// 10, R10-10b): "October 25 (November 1), 2026" reads
/// October 25, 2026, with the November 1 group
/// compared as garnish. The shape is pinned by what
/// follows the group — a comma and a year — and by the
/// line reading as a date once the group is taken out;
/// a group anywhere else is not tried here.
///
/// Round 37 (R37-F, F10 = red team RT37-8): a group
/// at this slot holding a bare day number and
/// nothing else is not garnish standing between a
/// month-day and its year — it IS the day, standing
/// where the wrapped twins read it ("(November
/// (8), 2026)" reads November 8 through the group
/// machinery, while the bare "November (8), 2026"
/// read nothing: the de-grouped candidate "November
/// , 2026" has no day to read). When the de-grouped
/// candidate reads nothing and the group's inner
/// text is a bare day number, the candidate is
/// rebuilt with the day standing in the group's
/// place and read again. The garnish reading above
/// is tried first and is untouched wherever it
/// reads, and a group holding anything else (a
/// word, a whole date) is never spliced.
///
/// Round 38 (R38-F, F8 = review R38-R3): the splice
/// stopped at a DOUBLED day group — "November
/// ((8)), 2026" and "November ([8]), 2026" read
/// nothing, while the wrapped twin "(November
/// ((8)), 2026)" reads November 8. The flat span
/// search passes a doubled group by construction
/// (a span whose inner text holds an opener of its
/// own pair is no flat span), so the group never
/// reached the splice. The slot now also walks the
/// balanced spans the flat search passes over, and
/// the day is read through the nesting
/// (`nested_bare_day_group_inner`): a group whose
/// whole inner text is itself a day group, in any
/// pair at either layer, is the day in two wrappers,
/// as the doubled MONTH groups already read at
/// their slot. The garnish arm is unchanged: it is
/// tried first, on the same de-grouped candidate,
/// for the balanced spans exactly as for the flat
/// ones.
pub(super) fn mid_date_group_reading(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    trailing_groups: &[SimpleDate],
    trailing_alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    // The flat spans, plus the balanced spans the
    // flat search passes over for holding a nested
    // opener (round 38, R38-F: the doubled groups),
    // in written order.
    let mut spans = parenthesized_spans(text);
    for span in balanced_spans(text) {
        if !spans.contains(&span)
            && text[span.0 + 1..span.1 - 1]
                .chars()
                .any(|c| matches!(c, '(' | '[' | '{'))
        {
            spans.push(span);
        }
    }
    spans.sort_unstable();
    for (start, end) in spans {
        let after = text[end..].trim_start();
        let Some(after_comma) = after.strip_prefix(',') else {
            continue;
        };
        let Some(year_token) = after_comma.split_whitespace().next() else {
            continue;
        };
        let year_clean = year_token.trim_matches(['.', ',']);
        if !(year_clean.len() == 2 || year_clean.len() == 4)
            || !year_clean.chars().all(|c| c.is_ascii_digit())
        {
            continue;
        }
        let mut candidate = String::with_capacity(text.len());
        candidate.push_str(&text[..start]);
        candidate.push_str(&text[end..]);
        // Round 11 (R11-6/R11-7 review shape): a
        // trailing group standing after the year is
        // garnish the composition already holds; the
        // de-grouped candidate is read on its core,
        // trailing groups off, so a mid-date group and
        // a trailing group together no longer defeat
        // each other and read as nothing.
        let candidate_core = strip_all_trailing_parenthesized(candidate.trim()).to_string();
        let candidate_raw = Some(candidate_core.clone());
        let (cand_date, _) = date_line_inner(&candidate_raw, today);
        let inner = &text[start + 1..end - 1];
        // Round 37 (R37-F, F10): the day-splice
        // candidate (see the function's head note) —
        // tried only where the de-grouped candidate
        // reads nothing, so every standing garnish
        // reading at this slot is untouched.
        let (candidate_core, read) = match cand_date {
            Some(read) => (candidate_core, read),
            None => {
                let Some(day) = nested_bare_day_group_inner(inner) else {
                    continue;
                };
                let mut spliced = String::with_capacity(text.len());
                spliced.push_str(&text[..start]);
                spliced.push_str(&day);
                spliced.push_str(&text[end..]);
                let spliced_core = strip_all_trailing_parenthesized(spliced.trim()).to_string();
                let spliced_raw = Some(spliced_core.clone());
                let (spliced_date, _) = date_line_inner(&spliced_raw, today);
                let Some(read) = spliced_date else {
                    continue;
                };
                (spliced_core, read)
            }
        };
        let mut all_groups: Vec<SimpleDate> = Vec::new();
        if let Some(d) = group_date(inner, today, anchor) {
            all_groups.push(d);
        }
        for g in trailing_groups {
            if !all_groups.contains(g) {
                all_groups.push(*g);
            }
        }
        let parenthesized_mismatch = mismatch_from_groups(&all_groups, read, false);
        let mut second_dates = second_dates_in_line(&candidate_core, today, read);
        for d in trailing_alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: Some(read),
            weekday_mismatch: None,
            parenthesized_mismatch,
            second_dates,
        });
    }
    None
}

/// A group standing at the month slot holding only
/// a month word is the month, standing where the
/// bare month stands (round 39, R39-F, F8 = red
/// team RT39-6): "(((November))) 1, 2026" read
/// nothing, while the doubled whole-date group and
/// the doubled day group at their own slots have
/// read since R38-F — the day at this very slot is
/// spliced by `mid_date_group_reading` when the
/// group holding it stands between a bare month
/// and its year. The month group had no splice at
/// any depth: the de-grouped core ("; 1, 2026")
/// names no month, and the group holding one
/// contributes no date of its own. When the
/// group's inner text, unwrapped layer by layer
/// through its nesting, is a month word and
/// nothing else, the candidate is rebuilt with
/// the month standing in the group's place and
/// read again — the same splice shape as the day
/// group's, at the date's other component slot.
/// The group may stand at the line's head or at a
/// Sunday-after phrase's base slot, the two places
/// a bare month stands in a date this reader
/// reads; a group holding anything else (a word, a
/// whole date, a day) is never spliced here.
pub(super) fn month_group_reading(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    trailing_groups: &[SimpleDate],
    trailing_alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    for (start, end) in balanced_spans(text) {
        let prefix = text[..start].trim();
        // The only nonempty month-slot prefixes
        // are the Sunday-after phrase's words —
        // three short tokens at most — so a longer
        // prefix is not the slot, and its length is
        // checked before any token is cleaned: a
        // line carrying thousands of groups must
        // not pay a prefix scan per group.
        let at_month_slot = prefix.is_empty()
            || (prefix.len() <= 32 && {
                let prefix_tokens: Vec<String> =
                    prefix.split_whitespace().map(group_token).collect();
                prefix_tokens == ["sunday".to_string(), "after".to_string()]
                    || prefix_tokens
                        == ["the".to_string(), "sunday".to_string(), "after".to_string()]
            });
        if !at_month_slot {
            continue;
        }
        let inner = &text[start + 1..end - 1];
        let Some(month) = nested_month_group_inner(inner) else {
            continue;
        };
        let mut spliced = String::with_capacity(text.len());
        spliced.push_str(&text[..start]);
        spliced.push_str(&month);
        spliced.push_str(&text[end..]);
        let spliced_core = strip_all_trailing_parenthesized(spliced.trim()).to_string();
        let spliced_raw = Some(spliced_core.clone());
        let (spliced_date, _) = date_line_inner(&spliced_raw, today);
        let Some(read) = spliced_date else {
            continue;
        };
        let mut all_groups: Vec<SimpleDate> = Vec::new();
        if let Some(d) = group_date(inner, today, anchor) {
            all_groups.push(d);
        }
        for g in trailing_groups {
            if !all_groups.contains(g) {
                all_groups.push(*g);
            }
        }
        let parenthesized_mismatch = mismatch_from_groups(&all_groups, read, false);
        let mut second_dates = second_dates_in_line(&spliced_core, today, read);
        for d in trailing_alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: Some(read),
            weekday_mismatch: None,
            parenthesized_mismatch,
            second_dates,
        });
    }
    None
}

/// The month word a month-slot group's inner text
/// holds through its nesting, when it holds one
/// and nothing else (round 39, R39-F, F8): the
/// inner text is unwrapped layer by layer — each
/// layer a balanced group standing as the whole
/// remaining text, in any of the three pairs, the
/// descent `nested_bare_day_group_inner` walks
/// for the day — until the word itself stands,
/// which is a month only when it is a month name
/// or abbreviation whole, letters and nothing
/// else. Returned as written.
/// Every matched opener of `text` with the byte
/// index of its closer, from one balancing walk
/// (round 39, R39-G, F9): the walk is
/// `balanced_spans`' own discipline — an opener
/// pushes, a closer that matches the stack top's
/// expected closer pops it and pairs the two — run
/// once over the whole text, so a reader that
/// descends a nesting layer by layer reads each
/// layer's span off the table instead of re-walking
/// the remaining text per layer. The pairing is
/// the walk's, pair for pair; only the bookkeeping
/// differs (every pair is kept, not only the
/// top-level spans).
fn group_match_closers(text: &str) -> Vec<Option<usize>> {
    let bytes = text.as_bytes();
    let mut matches: Vec<Option<usize>> = vec![None; bytes.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        if is_group_opener(b) {
            stack.push(i);
            continue;
        }
        if is_group_closer(b) {
            if let Some(&open) = stack.last() {
                if group_closer(bytes[open]) == b {
                    stack.pop();
                    matches[open] = Some(i);
                }
            }
        }
    }
    matches
}

fn nested_month_group_inner(inner: &str) -> Option<String> {
    // Round 39 (R39-G, F9): the descent reads its
    // layers off one match table (`group_match_closers`,
    // above) instead of calling `first_balanced_span`
    // per layer — that call re-walked the whole
    // remaining text each layer, so a group nested k
    // deep paid k walks of a shrinking remainder:
    // quadratic time in the nesting depth. The
    // descent itself is the standing one, step for
    // step: `cur` is `inner[off..off + cur.len()]`
    // throughout, trimmed at both ends; a layer
    // unwraps exactly when the opener at its head is
    // paired with the closer at its tail, which is
    // the span test the per-layer walk answered
    // (start 0, end the whole remainder).
    let matches = group_match_closers(inner);
    let mut off = inner.len() - inner.trim_start().len();
    let mut cur = inner.trim();
    loop {
        let first = cur.chars().next()?;
        if !matches!(first, '(' | '[' | '{') {
            break;
        }
        if matches[off] != Some(off + cur.len() - 1) {
            break;
        }
        let inside = &inner[off + 1..off + cur.len() - 1];
        off = off + 1 + (inside.len() - inside.trim_start().len());
        cur = inside.trim();
    }
    if !cur.is_empty() && cur.chars().all(|c| c.is_ascii_alphabetic()) && word_at_is_month(cur, 0) {
        Some(cur.to_string())
    } else {
        None
    }
}

/// The bare day number a mid-date group's inner text
/// holds, when it holds one and nothing else (round
/// 37, R37-F, F10): one or two digits naming 1..=31,
/// wearing at most the spoken suffix the flexible
/// reader itself honors ("8", "8th"), with the
/// group's own edge punctuation ("," / ".") off.
/// Returned as written, suffix included, so the
/// spliced candidate is the line the sender's words
/// make with the group unwrapped.
/// The bare day number a mid-date group's inner text
/// holds through its nesting, when it holds one
/// (round 38, R38-F, F8): the inner text is unwrapped
/// layer by layer — each layer a balanced group
/// standing as the whole remaining text, in any of
/// the three pairs — until the day group itself
/// stands, which `bare_day_group_inner` judges as it
/// judges the single group's. A layer holding
/// anything beside the nested group (a word, a
/// second group) stops the descent and is no day.
fn nested_bare_day_group_inner(inner: &str) -> Option<String> {
    // Round 39 (R39-G, F9): the same one-walk
    // descent as `nested_month_group_inner` above —
    // layers read off the match table, not re-walked
    // per layer.
    let matches = group_match_closers(inner);
    let mut off = inner.len() - inner.trim_start().len();
    let mut cur = inner.trim();
    loop {
        let first = cur.chars().next()?;
        if !matches!(first, '(' | '[' | '{') {
            break;
        }
        if matches[off] != Some(off + cur.len() - 1) {
            break;
        }
        let inside = &inner[off + 1..off + cur.len() - 1];
        off = off + 1 + (inside.len() - inside.trim_start().len());
        cur = inside.trim();
    }
    bare_day_group_inner(cur)
}

fn bare_day_group_inner(inner: &str) -> Option<String> {
    // Round 39 (R39-F, F8 = red team RT39-6): the
    // edge trim gains the sentence marks the group
    // cleaning itself sheds — a mark standing
    // inside the nesting, against the day number's
    // edge ("November ((8!)), 2026"), stopped the
    // descent's last step, where "November
    // ((8.)), 2026" has read since R38-F.
    let word = inner.trim().trim_matches([',', '.', '!', '?', ':', ';']);
    let digits = word
        .strip_suffix("st")
        .or_else(|| word.strip_suffix("nd"))
        .or_else(|| word.strip_suffix("rd"))
        .or_else(|| word.strip_suffix("th"))
        .unwrap_or(word);
    if digits.is_empty() || digits.len() > 2 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let day: u32 = digits.parse().ok()?;
    if (1..=31).contains(&day) {
        Some(word.to_string())
    } else {
        None
    }
}

/// Every distinct second parseable absolute date
/// standing in a date line (round 9, R9-13; round 10,
/// R10-4/R10-6): the scan reads the WHOLE line, before
/// the read date as well as after it. The first
/// date-shaped run in the line that is the read date
/// itself — or, on a Sunday-after line, the base the
/// read date resolves from — is the anchor and is
/// never a second date; every other distinct date the
/// line names, on either side of the anchor, is a
/// second date, in written order. Trailing
/// parenthesized groups are garnish, remembered on
/// their own records, and are not scanned here.
///
/// Round 11 (R11-7): the anchor is the read date's
/// OWN occurrence — the first run that is the read
/// date itself. A distinct date in the six days
/// before the read date shares its Sunday but is not
/// its proxy: it is a second date. On a Sunday-after
/// line the one other anchor is the resolution's
/// base — the date run the "Sunday after" phrase
/// itself consumes — and the base is never flagged
/// as its own second date.
///
/// Round 12 (R12-9a): the phrase consumes its run
/// wherever it stands in the line, not only where
/// the line's own reading found it. A phrase run's
/// value is its RESOLUTION: a second phrase whose
/// resolution differs from the read date is noted
/// with that resolution, exactly as the first
/// phrase's resolution is the read date itself. And
/// a plain run equal to the standing base — the base
/// the read date resolves from — is a restatement of
/// the base, never "another date", wherever in the
/// line it stands.
pub(super) fn second_dates_in_line(
    text: &str,
    today: SimpleDate,
    read: SimpleDate,
) -> Vec<SimpleDate> {
    // Round 18 (R18-1): the second-date scan cleans
    // with the one shared cleaning too (`group_token`),
    // so a "Sunday after" run wearing a colon or a
    // dash is the same run here as in a group.
    let core = strip_all_trailing_parenthesized(text.trim());
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    // Every run in the line, in written order: a
    // plain date run (base None), or a Sunday-after
    // phrase run whose date is its resolution and
    // whose base it consumed.
    struct LineRun {
        date: SimpleDate,
        base: Option<SimpleDate>,
    }
    let mut runs: Vec<LineRun> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some((base, end)) = sunday_after_phrase_run(&tokens, &cleaned, i, today) {
            runs.push(LineRun {
                date: first_sunday_after(base),
                base: Some(base),
            });
            i = end.max(i + 1);
            continue;
        }
        match parse_date_prefix(&tokens[i..], today) {
            Some((d, used)) => {
                runs.push(LineRun {
                    date: d,
                    base: None,
                });
                i += used.max(1);
            }
            None => i += 1,
        }
    }
    // The standing base: the base of the first
    // phrase whose resolution is the read date.
    let standing_base = runs
        .iter()
        .find(|r| r.base.is_some() && r.date == read)
        .and_then(|r| r.base);
    let mut own_anchored = false;
    let mut out: Vec<SimpleDate> = Vec::new();
    for run in runs {
        if run.base.is_some() {
            // A phrase resolving to the read date is
            // the anchor (or a restatement of it);
            // any other phrase is noted by its
            // resolution, its base consumed.
            if run.date != read && !out.contains(&run.date) {
                out.push(run.date);
            }
            continue;
        }
        if Some(run.date) == standing_base {
            continue;
        }
        if !own_anchored && run.date == read {
            own_anchored = true;
            continue;
        }
        if run.date != read && !out.contains(&run.date) {
            out.push(run.date);
        }
    }
    out
}

/// The Sunday-after phrase standing at token `i` of
/// a line, if one does (round 12, R12-9a): "Sunday
/// after" followed by a readable base date, the base
/// wearing its own leading weekday (behind an
/// optional "on") as the line reader allows.
/// Returns the base and the token index just past
/// the phrase.
pub(super) fn sunday_after_phrase_run(
    tokens: &[&str],
    cleaned: &[String],
    i: usize,
    today: SimpleDate,
) -> Option<(SimpleDate, usize)> {
    // Round 29 (R29-A, F8 = review MINOR-1): the
    // phrase is judged on the unglued vector, as
    // in `segment_tokens` — this run is also read
    // off cleanings that never unglued it (the
    // second-date scan's `group_token` cleaning),
    // so the phrase's two words are read unglued
    // here, at the shared reader itself.
    // Round 35 (R35-H): the shape's weekday may
    // stand wholly bracketed — the shared cleaning
    // reads a bracketed token as garnish, so its
    // cleaned form here is nothing, where the paren
    // twin's cleaning gives the word itself. The
    // raw token is asked for its bracketed word, so
    // "{Sunday} after ..." and "[Sunday] after ..."
    // are the phrase "(Sunday) after ..." is.
    if weekday_word(&unglued_word(&cleaned[i])) != Some(0)
        && bracketed_weekday_word(tokens[i]) != Some(0)
    {
        return None;
    }
    // Round 27 (R27-A, F3): the "after" is the
    // next word that is something after the
    // weekday, as at the scans that call here.
    let after_at = next_nonempty(cleaned, i + 1)?;
    if unglued_word(&cleaned[after_at]) != "after" {
        return None;
    }
    // Round 39 (R39-B, F3): a bracketed weekday
    // token standing between the weekday and the
    // "after" is a word standing there, as the
    // shape's own word and the base's leading
    // weekday already are (round 35, R35-H): the
    // shared cleaning reads "[Monday]" as garnish,
    // so the next-nonempty search above steps over
    // it and the phrase forms across it, where its
    // paren twin "(Monday)" cleans to the word
    // itself and no phrase forms — "Sunday [Monday]
    // after November 1, 2026" was totally silent
    // while the paren twin read on its plain scan.
    // A weekday word in brackets breaks the run
    // exactly as the word itself does.
    if tokens[i + 1..after_at]
        .iter()
        .any(|t| bracketed_weekday_word(t).is_some())
    {
        return None;
    }
    // Round 18 (R18-1): the one shared cleaning.
    fn clean(t: &str) -> String {
        group_token(t)
    }
    // Round 28 (R28-A, F2 = review MINOR-1): a
    // nothing-token standing between "after" and
    // its base ("the Sunday after \u{200b} November
    // 1, 2026") is nothing there — the base scan
    // starts on the first token that is something,
    // as `sunday_after_outcome`'s filtered scan
    // already reads it, and the skipped tokens
    // count in the span the scan reports.
    let skip = tokens[after_at + 1..]
        .iter()
        .take_while(|t| nothing_token(t))
        .count();
    let rest = &tokens[after_at + 1 + skip..];
    if let Some((base, used)) = parse_date_prefix(rest, today) {
        return Some((base, after_at + 1 + skip + used));
    }
    // The base's own leading weekday, wearing an
    // optional "on": strip it and read what it framed.
    // Round 35 (R35-H): the base's leading weekday
    // may stand wholly bracketed too, as the shape's
    // word may — the raw token is asked for its
    // bracketed word where the shared cleaning
    // gives nothing, so the braced and bracketed
    // framings strip exactly as the paren framing
    // does ("Sunday after {Monday} November 1, 2026"
    // consumes its base as its paren twin does).
    let lead_weekday = |t: &str| weekday_word(&clean(t)).or_else(|| bracketed_weekday_word(t));
    let offset = if !rest.is_empty() && lead_weekday(rest[0]).is_some() {
        1
    } else if rest.len() > 1
        && clean(rest[0]).eq_ignore_ascii_case("on")
        && lead_weekday(rest[1]).is_some()
    {
        2
    } else {
        return None;
    };
    let (base, used) = parse_date_prefix(&rest[offset..], today)?;
    Some((base, after_at + 1 + skip + offset + used))
}

/// The Sunday a bare relative phrase names,
/// resolved against `today` — the only anchor a
/// plain line carries at parse time (round 13,
/// R13-5): "this Sunday" and a bare "Sunday" name
/// the coming Sunday (today itself when today is
/// one), and "next Sunday" names the Sunday after
/// it — the reading under which "this" and "next"
/// name different days, as the sender's words do.
pub(super) fn relative_sunday_date(next: bool, today: SimpleDate) -> SimpleDate {
    let until_sunday = (7 - today.weekday()) % 7;
    let coming = add_days(today, until_sunday);
    if next {
        add_days(coming, 7)
    } else {
        coming
    }
}

/// The governor composition on a plain line's
/// signals (round 16, R16-4; round 17, R17-2: framed
/// and relative signals alike, each judged by the
/// words around its own token span): each signal is
/// judged exactly as a date occurrence inside a
/// group — its governor window (with the round-16
/// run spending and target claim), its trailing
/// governors, the split phrasal wrap, a contradicting
/// weekday — so the words between two dates on a
/// plain header line compose the way the same words
/// compose inside parentheses. Returns None when no
/// signal is governed (the standing first-signal
/// primacy then reads the line). When some signal
/// is governed, the first UNGOVERNED signal reads
/// and every other distinct date is named on the
/// second-date note; when every signal is governed
/// the line reads no date, the group's silence.
pub(super) fn plain_composition(
    raw_tokens: &[&str],
    tokens: &[String],
    signals: &[(SimpleDate, usize, usize)],
    alternatives: &[SimpleDate],
    today: SimpleDate,
) -> Option<DateLineReading> {
    let positional: Vec<(SimpleDate, usize, usize)> = signals
        .iter()
        .map(|(d, start, end)| (*d, *start, end - start))
        .collect();
    // Round 32 (R32-C): computed against one shared
    // scan of the line (`target_claims`).
    let claims: Vec<Option<TargetClaim>> = target_claims(raw_tokens, tokens, &positional);
    let any_target = claims.iter().any(|c| c.is_some());
    // Round 29 (R29-B, F7 = red team RT29-N1): the
    // plain surface of the group's naming rule —
    // the resolutions of Sundays the destination
    // stack consumed, read by the same naming-only
    // consultation `analyze_group` reads, over the
    // same Sunday positions (a bare Sunday at its
    // word, a relative phrase at its span's last
    // word, a trailing Sunday through the
    // crossing's trailing arm). A consumed Sunday
    // is the stack's own material and is named
    // nowhere beside the assertion it bounds, on
    // this surface exactly as in a group; a Sunday
    // the consultation does not consume — a
    // genuine alternative standing beside a
    // target in a reading with no destination
    // stack — is named as it always has been.
    let stack_consumed: Vec<SimpleDate> = positional
        .iter()
        .filter_map(|(d, start, used)| {
            let sunday_at = if tokens.get(*start).map(String::as_str) == Some("sunday") {
                Some(*start)
            } else if matches!(
                tokens.get(*start).map(String::as_str),
                Some("next") | Some("this")
            ) && *used >= 1
                && tokens.get(start + used - 1).map(String::as_str) == Some("sunday")
            {
                Some(start + used - 1)
            } else {
                None
            };
            sunday_at
                .filter(|s| weekday_consumed_in_destination_stack(raw_tokens, tokens, *s, today))
                .map(|_| *d)
        })
        .collect();
    // Round 18 (R18-2): the per-occurrence judgments
    // are kept apart — direct governance (a governor
    // before or after the occurrence, the split
    // phrasal wrap, a contradicting weekday) versus
    // the claim machinery — because governance now
    // attaches to the DATE VALUE, not the occurrence
    // (see the by-date pass after the loop).
    let mut direct_governed: Vec<bool> = Vec::new();
    let mut claim_governed: Vec<bool> = Vec::new();
    let mut last_target: Vec<bool> = Vec::new();
    // Round 18 (R18-5): each signal's "for" standing,
    // as in `analyze_group` — a for-occasion window
    // and a prospective "use" window.
    let mut for_occasions: Vec<bool> = Vec::new();
    let mut use_prospectives: Vec<bool> = Vec::new();
    for (idx, entry) in positional.iter().enumerate() {
        let (d, start, used) = *entry;
        let prev_end = if idx == 0 {
            0
        } else {
            positional[idx - 1].1 + positional[idx - 1].2
        };
        let window_start = governor_window_start(raw_tokens, tokens, prev_end, start, idx > 0);
        let window = &tokens[window_start..start.min(tokens.len())];
        for_occasions.push(
            passive_for_zone_start(&tokens[..window_start], window).is_some()
                || active_for_zone_start(window).is_some(),
        );
        // Round 28 (R28-A, F2): the window's last
        // word and the split pair's second word are
        // read over the words that are something, as
        // in `analyze_group` above.
        use_prospectives.push(last_word(window) == Some("use"));
        let next_start = positional.get(idx + 1).map(|(_, s, _)| *s);
        let split_first = last_word(window);
        let split_second = next_nonempty(tokens, start + used).map(|n| tokens[n].as_str());
        let split_phrasal = match (split_first, split_second) {
            (Some(first), Some(second)) => is_phrasal_governor_pair(first, second),
            _ => false,
        };
        let claim = &claims[idx];
        let before = match claim {
            Some(claim) => governed_before_target(window, window_start, claim),
            None => governed_before(&tokens[..window_start], window),
        };
        // Round 23 (R23-5): a framed date consumed
        // as a subject PP's object is governed for
        // this composition in the same way as in a
        // group — it asserts nothing of its own and
        // is named beside the date that stands.
        let before_disqualified = before
            || split_phrasal
            || contradicted_before(tokens, start, d)
            || date_in_subject_pp(tokens, start, used)
            // Round 24 (R24-7): the relative-phrase
            // twin — a relative phrase consumed as a
            // subject PP's object asserts nothing of
            // its own on the plain surface either, so
            // "the choir on next Sunday sang November
            // 8, 2026" reads November 8 with October
            // 25 named, not the other way round.
            || relative_in_subject_pp(tokens, start);
        let after_disqualified = governed_after(raw_tokens, tokens, start + used, next_start);
        let target_disqualified = claim.is_none()
            && any_target
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| c.is_some() && positional[j].0 != d);
        // Round 17 (R17-1): the last target wins on
        // the plain surface too — an earlier claimed
        // target superseded by a later distinct one
        // yields and is named, so the plain twin of
        // the collapsed group no longer asserts the
        // superseded date.
        let superseded_target = claim.is_some()
            && claims
                .iter()
                .enumerate()
                .any(|(j, c)| j > idx && c.is_some() && positional[j].0 != d);
        direct_governed.push(before_disqualified || after_disqualified);
        claim_governed.push(target_disqualified || superseded_target);
        last_target.push(claim.is_some() && !superseded_target);
    }
    // Round 18 (R18-2): governance by date value. A
    // date governed in ANY occurrence does not assert
    // from another occurrence — a cancelled date
    // restated in either form, on either side of its
    // governor, stays governed ("November 1, 2026,
    // cancelled, November 1, 2026" and, at a today
    // where "next Sunday" IS November 1, "November 1,
    // 2026, cancelled, next Sunday" both read
    // nothing). The one override: an occurrence a
    // target claim names as the LAST target asserts
    // by the claim, as a claimed target always has.
    // Restated UNGOVERNED signals are untouched —
    // with no directly governed occurrence the set
    // below is empty, this pass changes nothing, and
    // the R13-C first-signal reading stands.
    // Round 19 (R19-1): the by-date step binds only
    // a BARE restatement, exactly as in groups (see
    // `occurrence_is_bare`) — an occurrence heading
    // its own event clause on a plain line ("early
    // service November 1, 2026, late service
    // cancelled November 1, 2026") asserts by its
    // own judgment, and the plain twins of the
    // round-18 bare witnesses stay silent.
    let governed_values: Vec<SimpleDate> = positional
        .iter()
        .zip(direct_governed.iter())
        .filter(|(_, g)| **g)
        .map(|(entry, _)| entry.0)
        .collect();
    // Round 32 (R32-C, F3): the line's bracket-span
    // mask, computed once for the by-date pass, as
    // in `analyze_group`; the governed values stand
    // in a set beside their list there too.
    let span_mask = bracket_span_mask(raw_tokens);
    let governed_set: std::collections::BTreeSet<SimpleDate> =
        governed_values.iter().copied().collect();
    let mut governed: Vec<bool> = Vec::with_capacity(positional.len());
    for (idx, entry) in positional.iter().enumerate() {
        // Round 20 (R20-10): a Sunday-after phrase
        // signal is never bound by value, as in
        // groups (see `analyze_group`) — an
        // ungoverned phrase salvages by its own
        // resolution whatever its base's form; a
        // phrase governed in its own span still
        // dies by that judgment.
        let is_sunday_after_phrase = tokens.get(entry.1).map(String::as_str) == Some("sunday")
            && tokens.get(entry.1 + 1).map(String::as_str) == Some("after");
        let by_date = !direct_governed[idx]
            && governed_set.contains(&entry.0)
            && !last_target[idx]
            && !is_sunday_after_phrase
            && occurrence_is_bare_masked(tokens, &positional, idx, &span_mask);
        governed.push(direct_governed[idx] || claim_governed[idx] || by_date);
    }
    // Round 18 (R18-5): the prospective half of the
    // for-composition, on the plain surface too — a
    // "use" date standing after a for-occasion
    // asserts, with the for-date named: "it was
    // moved for November 1, 2026, use November 8,
    // 2026" asserts November 8. (The primacy half
    // needs no code here: an ungoverned plain line
    // already reads its first signal, and the
    // for-date stands first in its witness.) The
    // check runs before the no-governance return
    // below, because neither the for-date nor the
    // use date is governed — that is the shape's
    // whole point. A target claim anywhere on the
    // line fences it out, as in groups.
    if !any_target {
        let use_asserts = (0..positional.len()).rev().find(|i| {
            use_prospectives[*i]
                && !governed[*i]
                && (0..*i).any(|fi| for_occasions[fi] && !governed[fi])
        });
        if let Some(ui) = use_asserts {
            let read = positional[ui].0;
            let mut second_dates: Vec<SimpleDate> = Vec::new();
            for (d, _, _) in &positional {
                if *d != read && !stack_consumed.contains(d) && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            for d in alternatives {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            return Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch: None,
                second_dates,
            });
        }
    }
    if governed.iter().all(|g| !*g) {
        return None;
    }
    let mut qualified: Vec<SimpleDate> = Vec::new();
    for (idx, (d, _, _)) in positional.iter().enumerate() {
        if !governed[idx] && !qualified.contains(d) {
            qualified.push(*d);
        }
    }
    if qualified.is_empty() {
        // Every signal governed: the group's
        // silence, on the plain surface too.
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in alternatives {
            if !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: None,
            weekday_mismatch: None,
            parenthesized_mismatch: None,
            second_dates,
        });
    }
    let read = qualified[0];
    let mut second_dates: Vec<SimpleDate> = Vec::new();
    for (d, _, _) in &positional {
        if *d != read && !stack_consumed.contains(d) && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    for d in alternatives {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    Some(DateLineReading {
        date: Some(read),
        weekday_mismatch: None,
        parenthesized_mismatch: None,
        second_dates,
    })
}

/// One signal in a plain line, in written order
/// (round 13, R13-5; extracted round 18, R18-7 so
/// the salvaged-line naming in `date_line` scans a
/// line's signals with the very scan the plain
/// reading uses): a date, and whether it stood
/// framed (an absolute date as written) or relative
/// (a phrase, carried here by its resolution).
pub(super) struct PlainSignal {
    pub(super) date: SimpleDate,
    pub(super) framed: bool,
    // A pure relative phrase's resolution against
    // `anchor`, when this signal is one; None for a
    // framed signal and for a Sunday-after phrase
    // over a framed base (its date does not turn on
    // any anchor). Used only when the line proves
    // mixed, after the scan.
    pub(super) anchored: Option<SimpleDate>,
    // The token span the signal stands in (round
    // 16, R16-4): the governor composition judges a
    // signal by the words around its own span,
    // exactly as a group's occurrence is judged.
    pub(super) start: usize,
    pub(super) end: usize,
}

/// Every signal in a plain line's tokens, in
/// written order, with the round-14 anchor rule
/// applied: only a truly mixed line — a framed
/// signal standing beside a pure relative signal —
/// re-resolves its relative signals against
/// `anchor`; every other shape keeps the `today`
/// resolutions the scan computes.
/// Whether a bracketed weekday token stands in
/// the run of nothing-cleaning tokens directly
/// after token `i` (round 39, R39-B, F3): the run
/// the next-nonempty scans step over. A token in
/// it that names a weekday in brackets is a word
/// standing there (round 35, R35-H) — the weekday
/// word itself, as its paren twin's cleaning
/// gives it — not nothing.
fn bracketed_weekday_in_nothing_run(tokens: &[&str], cleaned: &[String], i: usize) -> bool {
    tokens[i + 1..]
        .iter()
        .zip(cleaned[i + 1..].iter())
        .take_while(|(_, c)| c.is_empty())
        .any(|(t, _)| bracketed_weekday_word(t).is_some())
}

pub(super) fn scan_plain_signals(
    tokens: &[&str],
    cleaned: &[String],
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<PlainSignal> {
    let mut signals: Vec<PlainSignal> = Vec::new();
    // Round 31 (R31-C, F4): the next-nonempty
    // answers for this scan, read off one table
    // (see `next_nonempty_table`) instead of
    // rescanning the nothing-runs per token.
    let nonempty_next = next_nonempty_table(cleaned);
    // The token index just past the last signal,
    // and whether that signal was a framed Sunday:
    // a bare "Sunday" standing directly after a
    // framed Sunday date is that date's trailing
    // weekday garnish, not a second signal.
    let mut last_end = 0usize;
    let mut last_framed_sunday = false;
    let mut i = 0;
    while i < tokens.len() {
        // A Sunday-after phrase consumes its run
        // first, as the second-date scan has it:
        // its base is never a separate signal.
        // Round 27 (R27-A, F3): the shape's words
        // are read over the nearest non-empty
        // tokens, as in the group scan above.
        let plain_after_at = nonempty_next[i + 1];
        if cleaned[i] == "sunday" && plain_after_at.is_some_and(|a| cleaned[a].as_str() == "after")
        {
            // "Sunday after next/this Sunday", the
            // relative base wearing an optional
            // "the": the resolution is the first
            // Sunday strictly after the relative
            // base's own resolution.
            let plain_after_at = plain_after_at.unwrap_or(i + 1);
            let first_at = nonempty_next[plain_after_at + 1];
            let (which_at, base_at) = match first_at {
                Some(f) if cleaned[f].as_str() == "the" => (
                    nonempty_next[f + 1],
                    nonempty_next[f + 1].and_then(|w| nonempty_next[w + 1]),
                ),
                other => (other, other.and_then(|w| nonempty_next[w + 1])),
            };
            let used = base_at.map(|b| b - i + 1).unwrap_or(0);
            let which = which_at.and_then(|w| cleaned.get(w)).map(String::as_str);
            let base_day = base_at.and_then(|b| cleaned.get(b)).map(String::as_str);
            if (which == Some("next") || which == Some("this")) && base_day == Some("sunday") {
                let base = relative_sunday_date(which == Some("next"), today);
                let date = first_sunday_after(base);
                let anchored_base = relative_sunday_date(which == Some("next"), anchor);
                let anchored = first_sunday_after(anchored_base);
                signals.push(PlainSignal {
                    date,
                    framed: false,
                    anchored: Some(anchored),
                    start: i,
                    end: i + used,
                });
                last_end = i + used;
                last_framed_sunday = false;
                i += used;
                continue;
            }
            if let Some((base, end)) = sunday_after_phrase_run(tokens, cleaned, i, today) {
                let date = first_sunday_after(base);
                signals.push(PlainSignal {
                    date,
                    framed: false,
                    anchored: None,
                    start: i,
                    end: end.max(i + 1),
                });
                last_end = end.max(i + 1);
                last_framed_sunday = false;
                i = end.max(i + 1);
                continue;
            }
        }
        // A bare relative phrase: "next Sunday" /
        // "this Sunday" (a framed date cannot stand
        // at "next"/"this", so no order is lost).
        let plain_rel_sunday = if cleaned[i] == "next" || cleaned[i] == "this" {
            nonempty_next[i + 1].filter(|&j| cleaned[j].as_str() == "sunday")
        } else {
            None
        };
        if let Some(rel_i) = plain_rel_sunday {
            // Round 25 (R25-B, F5b = review R25-2):
            // the object-continuation consultation,
            // as on the group surface above.
            if weekday_in_object_continuation_pp(tokens, cleaned, rel_i, today) {
                last_end = rel_i + 1;
                last_framed_sunday = false;
                i = rel_i + 1;
                continue;
            }
            let date = relative_sunday_date(cleaned[i] == "next", today);
            let anchored = relative_sunday_date(cleaned[i] == "next", anchor);
            signals.push(PlainSignal {
                date,
                framed: false,
                anchored: Some(anchored),
                start: i,
                end: rel_i + 1,
            });
            last_end = rel_i + 1;
            last_framed_sunday = false;
            i = rel_i + 1;
            continue;
        }
        match parse_date_prefix(&tokens[i..], today) {
            Some((d, used)) => {
                signals.push(PlainSignal {
                    date: d,
                    framed: true,
                    anchored: None,
                    start: i,
                    end: i + used.max(1),
                });
                last_end = i + used.max(1);
                last_framed_sunday = d.weekday() == 0;
                i += used.max(1);
            }
            None => {
                // A bare "Sunday" the framed reader
                // did not take (it framed no date
                // here) is a relative signal naming
                // the coming Sunday — unless it
                // stands directly after a framed
                // Sunday date, whose trailing
                // weekday garnish it is.
                // Round 21 (R21-7): the round-20
                // weekday-modifier rule (R20-6)
                // applies on the plain surface too —
                // a weekday word followed by a
                // noun-like word is that noun
                // phrase's modifier, never a relative
                // signal ("Sunday school, November 8,
                // 2026" files for November 8, not for
                // a phantom October 18), exactly as
                // in `dates_inside_group_signals`.
                // Round 39 (R39-B, F3): the modifier
                // judgment steps over the tokens the
                // cleaning reads as nothing — but a
                // bracketed weekday token in that run
                // is a word standing there (round 35,
                // R35-H), the weekday word itself, and
                // a weekday word leads no modifier
                // phrase: "Sunday [Monday] after
                // November 1, 2026" sees "monday"
                // after its Sunday exactly as its
                // paren twin does, so the judgment
                // does not hold across it.
                let modifier = weekday_modifier_at(cleaned, i)
                    && !bracketed_weekday_in_nothing_run(tokens, cleaned, i);
                if cleaned[i] == "sunday"
                    && !(i == last_end && last_framed_sunday)
                    // Round 23 (R23-6): the one
                    // weekday-modifier reading,
                    // verb-form adjective shape
                    // included ("Sunday left
                    // luggage"), as in groups.
                    && !modifier
                    // Round 22 (R22-5): a weekday
                    // consumed wholly as a subject
                    // PP's object ("the choir on
                    // Sunday was singing") spawns no
                    // signal on the plain surface
                    // either.
                    && !weekday_in_subject_pp(cleaned, i)
                    // Round 24 (R24-5): the object-
                    // continuation twin — a weekday
                    // PP inside a claim's object
                    // continuation is consumed on the
                    // plain surface too.
                    && !weekday_in_object_continuation_pp(tokens, cleaned, i, today)
                {
                    let date = relative_sunday_date(false, today);
                    let anchored = relative_sunday_date(false, anchor);
                    signals.push(PlainSignal {
                        date,
                        framed: false,
                        anchored: Some(anchored),
                        start: i,
                        end: i + 1,
                    });
                    last_end = i + 1;
                    last_framed_sunday = false;
                }
                i += 1;
            }
        }
    }
    // Round 14 (R14-6): the anchor adjustment,
    // as stated above.
    let is_mixed = signals.iter().any(|s| s.framed) && signals.iter().any(|s| s.anchored.is_some());
    if is_mixed {
        for s in &mut signals {
            if let Some(anchored) = s.anchored {
                s.date = anchored;
            }
        }
    }
    signals
}

/// The closed GARNISH-noun set of the lone-framed
/// fence (round 23, R23-7): the head nouns of the
/// pinned weekday-modifier shapes — printed matter
/// and the standing time-of-day / school garnish of
/// the r21b/r22a pins ("Sunday bulletins", "Sunday
/// school", "Sunday morning", and their
/// determiner- and modifier-wearing twins). The
/// set is closed as listed: a bare weekday + noun
/// phrase whose head stands outside it is content,
/// whatever the noun, and the fence does not fire.
///
/// Round 24 (R24-3): the set completes with the
/// printed-matter kin the lanes found missing in
/// the safe direction — "bulletin" (the singular
/// of its first member), "papers", "newsletter",
/// "leaflet", "sheets" — so "Sunday bulletin,
/// November 1, 2026" and "Sunday newsletter,
/// November 8, 2026" assert as their plural and
/// bulletins twins do. The rest of the kin stay
/// outside it as listed ("paper", "newsletters",
/// "leaflets", "sheet" are not members), and the
/// failure direction for an unlisted noun stays
/// silence.
pub(super) const GARNISH_NOUNS: [&str; 8] = [
    "bulletins",
    "school",
    "morning",
    "bulletin",
    "papers",
    "newsletter",
    "leaflet",
    "sheets",
];

/// Whether the lone-framed fence stands for a plain
/// line (round 21, R21-7; fenced round 22, R22-7;
/// inverted round 23, R23-7): the line's only signal
/// is the framed date at `start..end`, and beside
/// that date the line carries nothing but a
/// weekday-modifier phrase — a weekday word standing
/// with noun-like words, the framed date's garnish
/// ("Sunday bulletins", "printed Monday bulletins",
/// "Sunday school"). Every clause of the line (split
/// where a clause break stands) is judged: a clause
/// holding no words beside the date is garnish-free;
/// a clause holding words must hold a weekday-
/// modifier phrase and nothing but noun-like words
/// (and determiners) beside it.
///
/// Round 22's fence asked the clause to hold no
/// CONTENT head, judged against a ~70-word list —
/// and the list's failure direction was assertion:
/// every event noun the list missed ("Sunday
/// picnic", "Sunday brunch", "Sunday festival")
/// asserted its date, while a listed noun ("Sunday
/// rehearsal") stayed silent. Round 23 inverts the
/// test: the fence fires only when the phrase is
/// garnish by the closed `GARNISH_NOUNS` set — a
/// bare weekday + noun phrase fires only when its
/// head noun is on the set, and any other noun,
/// event or not, listed or not, blocks the fence.
/// The failure direction for an unlisted noun is
/// now silence, the safe direction, matching the
/// held rule for lone content lines. The
/// content-head list has left the fence entirely.
///
/// The verb-form adjective shape (R23-6) is judged
/// as the phrase it is: weekday + verb-form word +
/// noun-like word ("Sunday left luggage", "Sunday
/// baked beans") is a modifier phrase, exactly as
/// the signal scans read it. Round 25 (R25-A, F4 =
/// review R25-1): the fence judges that phrase by
/// its HEAD — its final noun — against
/// `GARNISH_NOUNS`, exactly as the bare-shape
/// branch does; the R23-6 shape pass is superseded
/// where the two conflict, so a verb-form phrase
/// whose head is not garnish ("Sunday baked beans
/// rehearsal", and the R23-6 pin "Sunday baked
/// beans" itself, head "beans") no longer carries
/// its date past the fence, while a garnish-headed
/// one ("Sunday baked bulletins") still does.
pub(super) fn lone_framed_fence(
    tokens: &[&str],
    cleaned: &[String],
    start: usize,
    end: usize,
) -> bool {
    let mut clauses: Vec<(usize, usize)> = Vec::new();
    let mut clause_start = 0usize;
    for (j, _) in tokens.iter().enumerate() {
        if clause_break_after_at(tokens, cleaned, j) {
            clauses.push((clause_start, j + 1));
            clause_start = j + 1;
        }
    }
    if clause_start < tokens.len() {
        clauses.push((clause_start, tokens.len()));
    }
    // Whether the word at `k` stands outside the
    // framed date's own span: the adjective shape's
    // head noun must be a real word of the clause,
    // not the date itself ("Monday meeting" is a
    // bare weekday + noun phrase whose head is
    // "meeting" — the date after it is no head).
    let outside_date = |k: usize| k < start || k >= end;
    let mut has_phrase = false;
    for (cs, ce) in clauses {
        let mut clause_has_phrase = false;
        let mut clause_has_words = false;
        for i in cs..ce {
            if i >= start && i < end {
                continue;
            }
            let w = &cleaned[i];
            if w.is_empty() {
                continue;
            }
            clause_has_words = true;
            if weekday_word(w).is_some() && i + 1 < ce && outside_date(i + 1) {
                // The verb-form adjective shape
                // first: weekday + verb-form word +
                // noun-like head outside the date
                // span ("Sunday left luggage").
                // Round 25 (R25-A, F4): the phrase
                // is judged by its HEAD — the final
                // noun of its noun run — against
                // `GARNISH_NOUNS`, exactly as the
                // bare-shape branch below judges its
                // phrases; the R23-6 pass by shape
                // alone is superseded where the two
                // conflict, so a stacked tail can no
                // longer ride a verb-form prefix past
                // the head test ("Sunday baked beans
                // rehearsal" reads nothing, as
                // "Sunday morning rehearsal" does).
                // Round 27 (R27-A, F3): the
                // phrase's words are the next words
                // that are something after the
                // weekday — a token that cleans to
                // nothing inside the phrase is
                // nothing in it, here and in the
                // bare shape below.
                let f1 = next_nonempty(cleaned, i + 1);
                let f2 = f1.and_then(|n| next_nonempty(cleaned, n + 1));
                if f1.is_some_and(|n| is_finite_verb_form(&cleaned[n]))
                    && f2.is_some_and(|n| noun_like_word(&cleaned[n]))
                    && f2.is_some_and(|n| n < ce && outside_date(n))
                {
                    let mut head = f2.unwrap_or(i + 2);
                    let mut k = head + 1;
                    while k < ce
                        && outside_date(k)
                        && (cleaned[k].is_empty() || noun_like_word(&cleaned[k]))
                    {
                        if noun_like_word(&cleaned[k]) {
                            head = k;
                        }
                        k += 1;
                    }
                    if !GARNISH_NOUNS.contains(&cleaned[head].as_str()) {
                        return false;
                    }
                    clause_has_phrase = true;
                    continue;
                }
                // The bare shape: weekday + noun
                // phrase — garnish only when the
                // phrase's HEAD is on the closed set
                // ("Sunday picnic" and "Sunday
                // rehearsal" alike read nothing now).
                // Round 24 (R24-3): the head is the
                // phrase's FINAL noun, not the word
                // after the weekday — a stacked
                // phrase passes only when its final
                // noun is garnish, so "Sunday morning
                // rehearsal", "Sunday school picnic",
                // "Sunday morning service", and
                // "Sunday morning meeting" read
                // nothing (their heads stand outside
                // the set), while "Sunday morning"
                // (head "morning"), "Sunday school
                // bulletin" (head "bulletin"), and
                // "Sunday morning bulletins" (head
                // "bulletins") assert. The verb-form
                // adjective shape above is routed
                // through the same garnish-head test
                // since round 25 (R25-A, F4).
                if f1.is_some_and(|n| noun_like_word(&cleaned[n])) {
                    let mut head = f1.unwrap_or(i + 1);
                    let mut k = head + 1;
                    while k < ce
                        && outside_date(k)
                        && (cleaned[k].is_empty() || noun_like_word(&cleaned[k]))
                    {
                        if noun_like_word(&cleaned[k]) {
                            head = k;
                        }
                        k += 1;
                    }
                    if !GARNISH_NOUNS.contains(&cleaned[head].as_str()) {
                        return false;
                    }
                    clause_has_phrase = true;
                    continue;
                }
            }
            // The verb-form adjective word of a
            // phrase starting at the word before it
            // ("left" in "Sunday left luggage"):
            // the phrase's own material.
            if i > cs
                && prev_nonempty(cleaned, i, 1)
                    .first()
                    .is_some_and(|&p| weekday_word(&cleaned[p]).is_some())
                && is_finite_verb_form(w)
                && next_nonempty(cleaned, i + 1)
                    .is_some_and(|n| noun_like_word(&cleaned[n]) && outside_date(n))
            {
                continue;
            }
            // Round 24 (R24-8): a possessive
            // determiner standing before the weekday
            // phrase ("our Sunday bulletins", "my
            // Sunday bulletins", "their Sunday
            // bulletins") is the phrase's own material
            // exactly as a plain determiner is — it
            // does not block the fence, and the phrase
            // is still judged by its head noun above.
            if noun_like_word(w)
                || matches!(w.as_str(), "the" | "a" | "an")
                || POSSESSIVE_DETERMINERS.contains(&w.as_str())
            {
                continue;
            }
            return false;
        }
        if clause_has_words && !clause_has_phrase {
            return false;
        }
        has_phrase = has_phrase || clause_has_phrase;
    }
    has_phrase
}

/// A plain line naming two or more framed dates
/// and reading in no other way (round 12, R12-6):
/// "October 25, 2026 and November 1, 2026" used to
/// read as naming no Sunday at all. First-date
/// primacy — the standing rule inside groups — is
/// the line's rule too: the first framed date is
/// the line's date, and the remaining DISTINCT
/// dates join the second-date note. The same date
/// written twice is two runs of one date: it reads
/// as that date plainly, with nothing noted. A line
/// naming fewer than two date runs is not this
/// shape and keeps its standing readings.
///
/// Round 13 (R13-5): the reading covers MIXED
/// signals — a framed date and a relative phrase
/// ("next Sunday", "this Sunday", a bare "Sunday",
/// "the Sunday after next Sunday", or a Sunday-
/// after phrase, whose resolution the second-date
/// scan already computes) in either order. The
/// FIRST signal is the line's date, the standing
/// primacy: a first framed date reads as itself,
/// and a first relative phrase reads as its
/// resolution against `today`. Every other
/// distinct signal date joins the second-date
/// note — a relative phrase's resolution named as
/// a second date, or a framed date named beside
/// the resolution. The same signal twice — the
/// framed date restated, a relative phrase whose
/// resolution IS the framed date, or the same
/// relative phrase twice — reads plainly, with
/// nothing noted, as the framed twin already
/// does. A line whose only signals are relative
/// phrases naming DIFFERENT Sundays is not this
/// shape: a bare relative line keeps its standing
/// in-words reading, anchored on the bulletin.
///
/// Round 14 (R14-6): a MIXED line's relative
/// phrases resolve against `anchor` — the
/// contribution file's own date, when the parse
/// carries one, else `today` (the caller has
/// already clamped it: never after `today`, and
/// `today` itself when no usable file date
/// stands). The anchor applies ONLY here and
/// ONLY when the line is truly mixed (at least
/// one framed signal and at least one pure
/// relative signal): framed signals still read
/// as written against `today`, a Sunday-after
/// phrase over a framed base still resolves off
/// that base, and a line with no framed signal
/// keeps its standing relative-only reading
/// against `today`. When anchor and today agree
/// the reading is identical to the round-13 one.
pub(super) fn plain_multi_date_reading(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    // Round 18 (R18-1): ONE token cleaning on every
    // surface — the plain path cleans its tokens with
    // `group_token`, exactly as the group path does.
    // The plain cleaner used to leave ":" on its
    // tokens, so a governor wearing a colon
    // ("cancelled: November 1, 2026, next Sunday")
    // governed inside parentheses and was invisible
    // on a plain line, and the cancelled date armed.
    // The group set also strips "-" and the en/em
    // dashes, which the plain set left on.
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&tokens);
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.len() < 2 {
        // Round 21 (R21-7): a lone FRAMED signal
        // whose line carries a weekday-modifier
        // phrase ("Sunday school, November 8,
        // 2026", "printed Sunday bulletins,
        // November 8, 2026") still reads — the
        // modifier phrase is the framed date's own
        // garnish (see `scan_plain_signals`: a
        // weekday word followed by a noun-like
        // word is a modifier, never a relative
        // signal), and with the phantom signal
        // gone the framed date is the line's only
        // signal and stands by the same primacy,
        // the composition below judging its
        // governance first as it does for two.
        // A lone framed signal under any other
        // words keeps its standing readings (a
        // content-headed line alone reads
        // nothing), and a lone relative signal
        // keeps the in-words readings.
        let lone_framed = signals.len() == 1 && signals[0].framed;
        // Round 22 (R22-7): the fence fires only when the
        // weekday-modifier phrase is the line's only
        // content beside the framed date — no clause of
        // the line may carry other content: "Monday
        // meeting, service November 8, 2026", "service
        // November 8, 2026, Monday meeting", and
        // "Tuesday choir, November 1, 2026" all read
        // nothing, where the round-21 fence's any-
        // weekday-anywhere scan asserted them. Round 23
        // (R23-7): the phrase must be garnish by the
        // closed set in `lone_framed_fence` — the
        // content-head list that judged this in round
        // 22 has left the fence.
        let modifier_phrase =
            lone_framed && lone_framed_fence(&tokens, &cleaned, signals[0].start, signals[0].end);
        if !(lone_framed && modifier_phrase) {
            return None;
        }
    }
    let framed_count = signals.iter().filter(|s| s.framed).count();
    if framed_count == 0 {
        // No framed signal: only the restated
        // relative phrase — every signal resolving
        // to one date — reads, and it reads
        // plainly. Differing relative phrases keep
        // the standing readings.
        let read = signals[0].date;
        if signals.iter().any(|s| s.date != read) {
            return None;
        }
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: Some(read),
            weekday_mismatch: None,
            parenthesized_mismatch: None,
            second_dates,
        });
    }
    // Round 16 (R16-4): a line whose signals are all
    // framed composes its governors like a group —
    // "November 1, 2026, postponed, drop it,
    // November 8, 2026" asserts November 8, November
    // 1 governed and named. Round 17 (R17-2): the
    // composition reaches MIXED lines too — a
    // relative phrase DOES have a token span (the
    // phrase's own words), so every signal, framed
    // or relative, is judged by the words around its
    // span: "November 1, 2026, cancelled, next
    // Sunday" asserts October 25 (the relative's
    // resolution, resolved exactly as round 13
    // resolved it, the anchor rules unchanged) with
    // November 1 governed and named. The composition
    // returns None when no signal is governed, so a
    // mixed line with no governor phrase, run, or
    // target claim keeps the standing first-signal
    // reading below byte-for-byte, and single-date
    // plain lines never reach this reading at all.
    if framed_count > 0 {
        let spans: Vec<(SimpleDate, usize, usize)> =
            signals.iter().map(|s| (s.date, s.start, s.end)).collect();
        if let Some(reading) = plain_composition(&tokens, &cleaned, &spans, alternatives, today) {
            return Some(reading);
        }
    }
    let read = signals[0].date;
    let mut second_dates: Vec<SimpleDate> = Vec::new();
    for s in &signals[1..] {
        if s.date != read && !second_dates.contains(&s.date) {
            second_dates.push(s.date);
        }
    }
    for d in alternatives {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    Some(DateLineReading {
        date: Some(read),
        weekday_mismatch: None,
        parenthesized_mismatch: None,
        second_dates,
    })
}

/// The governor composition's reading of a plain
/// line's core, alone (round 18, R18-7): the same
/// scan and the same composition
/// `plain_multi_date_reading` uses, but WITHOUT the
/// first-signal fallback — None when no signal is
/// governed, so a caller can tell a composition
/// reading from a primacy reading and take only the
/// composition's naming.
pub(super) fn plain_composition_reading(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&tokens);
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.len() < 2 || !signals.iter().any(|s| s.framed) {
        return None;
    }
    let spans: Vec<(SimpleDate, usize, usize)> =
        signals.iter().map(|s| (s.date, s.start, s.end)).collect();
    plain_composition(&tokens, &cleaned, &spans, alternatives, today)
}

/// The date values a plain core governs when its
/// composition governs EVERY signal it carries
/// (round 20, R20-1): the distinct signal dates of
/// a core whose `plain_composition` reading is the
/// every-signal-governed silence. A core that
/// asserts, or governs nothing, or carries no
/// signal, governs no value for this purpose — an
/// asserting core composes with its groups through
/// the standing garnish comparison, not through
/// this rule. The composition itself is the judge:
/// a single governed date ("November 1, 2026,
/// cancelled") and a multi-signal all-governed
/// core are read by the same scan the plain
/// reading uses, so no second governor test
/// stands beside it.
pub(super) fn every_governed_core_values(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&tokens);
    let mut signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.is_empty() {
        return Vec::new();
    }
    // Round 21 (R21-4): every value this composition
    // binds resolves its relative signals at the parse
    // anchor — the same anchor the groups use since
    // R19-3. The plain scan re-resolves relatives at
    // the anchor only on a MIXED line, so a governed
    // core whose value is a relative phrase alone
    // ("next Sunday, cancelled") kept its scan-day
    // resolution (November 1 on an October 23 scan of
    // an October 16 file) and never met the group
    // restating its anchored value (October 25): the
    // group asserted the cancelled date and the note
    // named a Sunday nobody wrote. Here, where the
    // values are read off for governance binding,
    // the anchor applies to every relative signal.
    for s in &mut signals {
        if let Some(anchored) = s.anchored {
            s.date = anchored;
        }
    }
    let spans: Vec<(SimpleDate, usize, usize)> =
        signals.iter().map(|s| (s.date, s.start, s.end)).collect();
    let no_alternatives: &[SimpleDate] = &[];
    match plain_composition(&tokens, &cleaned, &spans, no_alternatives, today) {
        Some(reading) if reading.date.is_none() => {
            let mut values: Vec<SimpleDate> = Vec::new();
            for s in &signals {
                if !values.contains(&s.date) {
                    values.push(s.date);
                }
            }
            values
        }
        _ => Vec::new(),
    }
}

/// One composition for a governed plain core and
/// its trailing groups (round 20, R20-1): the
/// reading `date_line` takes when the core's own
/// composition reads no date because every core
/// signal is governed, and trailing groups stand
/// beside it. Before this, the two paths split on
/// exactly this shape: the For Sunday reading
/// returned the core's silence and dropped the
/// groups unnamed, while the Keep-through
/// fallback asserted the last group blind — in
/// the base witness asserting the very date the
/// core cancelled.
///
/// The composition is the paren-only composition
/// with the core standing where a sibling group
/// stands in the cross-group rule (round 19,
/// R19-2): the core's governed values bind a
/// trailing group's BARE restatement of the same
/// value — governance crosses the core/group
/// boundary exactly as it crosses a group
/// boundary, and binds exactly the same
/// occurrences (a labelled group, a claim's
/// target, and a group governed in itself are
/// judged as they always were). A surviving
/// group then stands by the trailing-group
/// primacy the paren-only path uses: the last
/// surviving group is the line's date, earlier
/// surviving groups disagree with it on the
/// mismatch record, a chain's superseded targets
/// and a two-group relative loser are named on
/// the note as the paren-only path names them,
/// and the core's governed values are named on
/// the note beside the read date, as the plain
/// composition names its governed dates. When
/// no group survives, the line reads nothing and
/// names only the groups' alternatives, as a
/// no-reading line does. None when the core is
/// not an every-governed core, or no trailing
/// group stands at all: every other shape keeps
/// its standing reading.
pub(super) fn governed_core_group_reading(
    text: &str,
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    alternatives: &[SimpleDate],
    core_mismatch: Option<WeekdayMismatch>,
) -> Option<DateLineReading> {
    let governed_values = every_governed_core_values(core, today, anchor);
    if governed_values.is_empty() {
        return None;
    }
    let analyses = trailing_group_analyses(text, today, anchor);
    if analyses.is_empty() {
        return None;
    }
    // The core's governance applied to the
    // groups: a bare contribution of a governed
    // value is suppressed; every other
    // contribution stands as the cross-group pass
    // left it.
    let surviving: Vec<SimpleDate> = analyses
        .iter()
        .filter_map(|(_, a)| {
            a.contributed
                .filter(|d| !(a.contributed_bare && governed_values.contains(d)))
        })
        .collect();
    let Some(read) = surviving.last().copied() else {
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in alternatives {
            if !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return Some(DateLineReading {
            date: None,
            weekday_mismatch: core_mismatch,
            parenthesized_mismatch: None,
            second_dates,
        });
    };
    // A split chain's superseded targets are
    // named on the note, not compared as garnish,
    // as on the paren-only path — counting only
    // targets that survived the core's governance.
    let superseded: Vec<SimpleDate> = chain_superseded_dates(text, today, anchor, read)
        .into_iter()
        .filter(|d| surviving.contains(d))
        .collect();
    let remaining: Vec<SimpleDate> = surviving
        .iter()
        .copied()
        .filter(|d| !superseded.contains(d))
        .collect();
    let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
    let mut second_dates: Vec<SimpleDate> = Vec::new();
    for d in &governed_values {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    for d in alternatives {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    for d in &superseded {
        if *d != read && !second_dates.contains(d) {
            second_dates.push(*d);
        }
    }
    // Round 19 (R19-10)'s loser naming, generalized
    // to multi-group readings by round 20 (R20-9),
    // as the paren-only path applies it.
    if surviving.len() >= 2
        && analyses
            .iter()
            .any(|(inner, _)| group_inner_is_relative(inner))
    {
        for d in &surviving {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
    }
    Some(DateLineReading {
        date: Some(read),
        weekday_mismatch: None,
        parenthesized_mismatch,
        second_dates,
    })
}

/// What a date line's plain core — the line with
/// every parenthesized group taken out, wherever
/// the groups stood — contributes to the whole-line
/// composition (round 21, R21-1/R21-2/R21-3).
pub(super) enum CoreProfile {
    /// The core asserts a date: by the inner readers
    /// (a framed or Sunday-after core), by the plain
    /// composition / first-signal primacy, or — a
    /// single framed signal headed by its own words —
    /// as a content-headed core ("service November 1,
    /// 2026", "choir November 1, 2026"), whose date
    /// round 20's composition never reached because
    /// no plain reader asserts a lone content-headed
    /// signal. `single_signal` marks that last family:
    /// it is the family the composition extends to,
    /// beside groups, for the first time.
    Assert {
        read: SimpleDate,
        second_dates: Vec<SimpleDate>,
        single_signal: bool,
    },
    /// The core asserts nothing and holds values the
    /// composition binds by: an every-governed core's
    /// governed values, or a for-occasion core's
    /// occasion date ("it was scheduled for November
    /// 1, 2026", "we moved the meeting for November 1,
    /// 2026") — the occasion date is the core's value
    /// for governance binding, while the core alone
    /// keeps reading nothing, as it always has.
    Governed {
        values: Vec<SimpleDate>,
        for_occasion: bool,
    },
}

/// A plain core's composition profile, read with the
/// standing readers in their standing order — the
/// inner readers, the plain multi-date reading, the
/// every-governed values — and, for a lone framed
/// signal none of them asserts or governs, the
/// signal's own standing: a for-occasion (its window
/// ends in a passive or active "for" construction)
/// contributes its occasion date as a governed
/// value; any other lone framed signal is a
/// content-headed core and asserts its date. A
/// weekday mismatch on the core, a relative-only
/// core, and a core no reader reaches have no
/// profile: the whole-line composition does not
/// touch them, and their standing readings stand.
pub(super) fn core_composition_profile(
    core: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<CoreProfile> {
    let core_raw = Some(core.to_string());
    let (inner_date, inner_mismatch) = date_line_inner(&core_raw, today);
    if inner_mismatch.is_some() {
        return None;
    }
    if let Some(read) = inner_date {
        let mut second_dates = second_dates_in_line(core, today, read);
        // Round 18 (R18-7)'s merge, as in `date_line`:
        // the plain composition's naming joins the
        // note when it reads the same date.
        if let Some(plain) = plain_composition_reading(core, today, anchor, &[]) {
            if plain.date == Some(read) {
                let consumed_bases = sunday_after_consumed_bases(core, today);
                for d in &plain.second_dates {
                    if *d != read && !consumed_bases.contains(d) && !second_dates.contains(d) {
                        second_dates.push(*d);
                    }
                }
            }
        }
        let profile = CoreProfile::Assert {
            read,
            second_dates,
            single_signal: false,
        };
        return Some(profile);
    }
    if let Some(reading) = plain_multi_date_reading(core, today, anchor, &[]) {
        if let Some(read) = reading.date {
            let profile = CoreProfile::Assert {
                read,
                second_dates: reading.second_dates,
                single_signal: false,
            };
            return Some(profile);
        }
        // An every-signal-governed silence: its
        // governed values are read off below.
    }
    let governed_values = every_governed_core_values(core, today, anchor);
    if !governed_values.is_empty() {
        let profile = CoreProfile::Governed {
            values: governed_values,
            for_occasion: false,
        };
        return Some(profile);
    }
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&tokens);
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.len() != 1 || !signals[0].framed {
        return None;
    }
    let signal = &signals[0];
    let window_start = governor_window_start(&tokens, &cleaned, 0, signal.start, false);
    let window = &cleaned[window_start..signal.start.min(cleaned.len())];
    let for_occasion = passive_for_zone_start(&cleaned[..window_start], window).is_some()
        || active_for_zone_start(window).is_some();
    if for_occasion {
        let profile = CoreProfile::Governed {
            values: vec![signal.date],
            for_occasion: true,
        };
        return Some(profile);
    }
    let profile = CoreProfile::Assert {
        read: signal.date,
        second_dates: Vec::new(),
        single_signal: true,
    };
    Some(profile)
}

/// The whole-line composition (round 21, R21-1 /
/// R21-2 / R21-3): ONE composition for every segment
/// of a date line, in written order — the R20-1
/// composition, which covered a governed plain core
/// and its TRAILING groups, extended to the whole
/// line. The line's plain core is the line with
/// every parenthesized group taken out, wherever
/// the groups stood, and every group on the line —
/// leading, trailing, or standing between plain
/// segments — composes with that core by the same
/// rules the trailing groups compose by:
///
/// - Beside an ASSERTING core, every group is
///   garnish: a group that disagrees with the read
///   date is named on the mismatch record, a group
///   that agrees is silent — leading groups exactly
///   as trailing ones ("(November 8) November 15,
///   2026" asserts November 15 and names November 8;
///   "(November 15) November 15, 2026" asserts
///   November 15, silently). A plain segment after
///   the last group is part of the core, so a
///   post-group date is judged by the plain
///   composition itself, never by a first-signal
///   reading that displaces the groups.
/// - Beside a GOVERNED core (an every-governed core,
///   or a for-occasion core by its occasion date),
///   the core's values bind a group's BARE
///   restatement of the same value, wherever the
///   group stands; a surviving group asserts by the
///   paren-only primacy (the last surviving group
///   stands, earlier survivors named on the mismatch
///   record), and the core's values are named on the
///   note beside the read date. When no group
///   survives, the line reads nothing.
///
/// The composition is entered only for the shapes
/// the standing readings do not already compose: a
/// group standing anywhere but the trailing run, or
/// a lone content-headed / for-occasion core beside
/// its trailing groups. Every other shape — the
/// paren-only line, the relative in-words line, the
/// mid-date group, a framed or governed core beside
/// trailing groups alone — keeps its standing
/// reading byte for byte, and this returns None.
/// Round 25 (R25-B, M3 = red team RT25-7): whether
/// the de-grouped `core`'s assertion of `read` is a
/// BRACKETED BARE RESTATEMENT: the asserting signal
/// is framed, bare (R17-2/R24-3: no claiming or
/// governing words of its own, and no sibling signal
/// on the core asserts an ungoverned value), and
/// its occurrence is wrapped in square brackets or
/// braces — glued to its first/last token ("[Novem-
/// ber 1]", "[[November 1]]"), doubled, or standing
/// as their own tokens ("[ November 1 ]"). Such a
/// restatement carries no reading of its own when
/// a group on the line governs the same value
/// directly. A bracketed occurrence that is itself
/// a claim, and a lone "[November 1]" with no
/// governing group (the recorded alone-unread
/// limitation's other half), are untouched.
pub(super) fn core_bracketed_bare_restatement(
    core: &str,
    read: SimpleDate,
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    let raw_tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&raw_tokens);
    let signals = scan_plain_signals(&raw_tokens, &cleaned, today, anchor);
    // Round 26 (R26-A, F1): the spans stand in
    // `occurrence_is_bare`'s own convention — (date,
    // start, USED), the occurrence's own token count,
    // converted from the signal's end exactly as
    // `plain_composition` converts them. As shipped
    // (R25-B) this passed each signal's region END —
    // the next signal's start, or the line's length —
    // where the count belonged: the bare test then
    // placed the occurrence's end at start + end,
    // past the next occurrence's start whenever this
    // occurrence did not stand at token 0, its
    // after-region slice inverted, and the parse
    // panicked on any governed line whose core
    // carried two signals ("service November 1, 2026
    // (November 1, cancelled) next Sunday"). The
    // scan's signals are written-ordered and
    // non-overlapping, so the true spans are
    // well-ordered by construction: each occurrence
    // ends at its own end, no later than the next
    // occurrence's start — the R21-5 bounding rule
    // the bare test assumes of every caller.
    let spans: Vec<(SimpleDate, usize, usize)> = signals
        .iter()
        .map(|s| (s.date, s.start, s.end - s.start))
        .collect();
    // Round 32 (R32-C, F3): the core's bracket-span
    // mask, computed once for the bare tests below,
    // as in `analyze_group`.
    let span_mask = bracket_span_mask(&raw_tokens);
    signals.iter().enumerate().any(|(idx, s)| {
        if s.date != read || !s.framed {
            return false;
        }
        if !occurrence_is_bare_masked(&cleaned, &spans, idx, &span_mask) {
            return false;
        }
        let open = raw_tokens[s.start]
            .trim_start_matches(';')
            .starts_with(['[', '{'])
            || (s.start > 0
                && matches!(
                    raw_tokens[s.start - 1].trim_matches(';'),
                    "[" | "{" | "[[" | "{{"
                ));
        let close = raw_tokens[s.end - 1].ends_with([']', '}'])
            || (s.end < raw_tokens.len() && matches!(raw_tokens[s.end], "]" | "}" | "]]" | "}}"));
        // Round 26 (R26-C, M2 = review R26-6): an
        // UNCLOSED span binds the same restatement —
        // the bracket opened before the date and
        // never closed ("(November 1, cancelled)
        // [our choir) November 1": the span opens
        // over the words before the date and runs
        // unclosed through the end of the core).
        // The open side counts any span still open
        // at the date's own start; the close side
        // counts a span still unclosed at the end of
        // the core. A span that closed BEFORE the
        // date ("[our choir] November 1") opens
        // nothing over it and asserts as it always
        // has — that is content standing beside the
        // governed sibling, not a restatement of it.
        let depth_at = |upto: usize| -> i32 {
            raw_tokens[..upto]
                .iter()
                .map(|t| {
                    let o = t.chars().filter(|c| *c == '[' || *c == '{').count() as i32;
                    let c = t.chars().filter(|c| *c == ']' || *c == '}').count() as i32;
                    o - c
                })
                .sum()
        };
        let open = open || depth_at(s.start) > 0;
        let close = close || depth_at(raw_tokens.len()) > 0;
        open && close
    })
}

pub(super) fn whole_line_composition(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<DateLineReading> {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return None;
    }
    // The mid-date group ("October 25 (November 1),
    // 2026") is the mid-date reading's shape, not a
    // segment boundary: a group followed by a comma
    // and a year belongs to the date it stands in.
    for (_, end) in &spans {
        let after = text[*end..].trim_start();
        if let Some(after_comma) = after.strip_prefix(',') {
            let year_token = after_comma.split_whitespace().next().unwrap_or("");
            let year_clean = year_token.trim_matches(['.', ',']);
            if (year_clean.len() == 2 || year_clean.len() == 4)
                && year_clean.chars().all(|c| c.is_ascii_digit())
            {
                return None;
            }
        }
    }
    // The trailing run: the groups the standing
    // readings already compose with the core.
    // Round 40 (R40-B, F1): the run's length is
    // the line scan's split count — the same run
    // the loop here used to count one strip at a
    // time.
    let trailing_run = LineScan::new(text).trailing_splits().len();
    let has_non_trailing_group = spans.len() > trailing_run;
    // The core: the line with every group taken out.
    // A group leaves a clause break behind it — a
    // semicolon token, which the date readers treat
    // as a break and never absorb into a date run
    // the way a comma token is absorbed: the group
    // is a segment boundary, so a governor standing
    // before it ("cancelled (November 8) November 8,
    // 2026") is spent where it stood and does not
    // reach across the group to govern the plain
    // segment after it — that segment is judged on
    // its own words, as a segment.
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        // Round 39 (R39-B, F3): the break stands
        // as its own token, as it does when the
        // group stood spaced — a group glued
        // against the word before it ("Sunday
        // [November 8, 2026] after ..." written
        // "Sunday[November 8, 2026] after ...")
        // must not glue the break to that word
        // either, or the core's readers see one
        // token "Sunday;" where the spaced twin's
        // core gives them "Sunday" and a break,
        // and the phrase the spaced core leaves
        // unformed forms and asserts. A group
        // glued on BOTH sides ("Sunday[November
        // 8, 2026]after ...") welds its neighbours
        // in the sender's own bytes and keeps its
        // standing core: that dress is judged by
        // its own readers, not this composition.
        let right_spaced = text[*end..]
            .chars()
            .next()
            .map_or(true, char::is_whitespace);
        if right_spaced && !core.is_empty() && !core.ends_with(char::is_whitespace) {
            core.push(' ');
        }
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    let core = core.trim();
    let profile = match core_composition_profile(core, today, anchor) {
        Some(profile) => profile,
        None => {
            // Round 22 (R22-1): a groups-only line whose
            // groups are separated by a semicolon, a dash,
            // a comma, or plain words — even a filler word
            // like "note" — composes exactly as the same
            // groups standing adjacent do. The de-grouped
            // core carries no signal of its own (no date,
            // no relative phrase), so there is no core
            // profile; the line is the paren-only
            // composition over every group on the line,
            // in written order, and the Keep-through
            // fallback (which reads this composition
            // through `trailing_dates_after_core_governance`)
            // and the For Sunday reading agree on it.
            let core_tokens: Vec<&str> = core.split_whitespace().collect();
            let core_cleaned: Vec<String> = segment_tokens(&core_tokens);
            let core_signals = scan_plain_signals(&core_tokens, &core_cleaned, today, anchor);
            if !core_signals.is_empty() {
                return None;
            }
            let entries = all_group_analyses(text, today, anchor);
            let mut alternatives: Vec<SimpleDate> = Vec::new();
            for (_, a) in &entries {
                for d in &a.alternatives {
                    if !alternatives.contains(d) {
                        alternatives.push(*d);
                    }
                }
            }
            let contributed: Vec<SimpleDate> =
                entries.iter().filter_map(|(_, a)| a.contributed).collect();
            let Some(read) = contributed.last().copied() else {
                return Some(DateLineReading {
                    date: None,
                    weekday_mismatch: None,
                    parenthesized_mismatch: None,
                    second_dates: alternatives,
                });
            };
            let superseded = chain_superseded_dates(text, today, anchor, read);
            let remaining: Vec<SimpleDate> = contributed
                .iter()
                .copied()
                .filter(|d| !superseded.contains(d))
                .collect();
            let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
            let mut second_dates: Vec<SimpleDate> = Vec::new();
            for d in &alternatives {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            for d in &superseded {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            if contributed.len() >= 2
                && entries
                    .iter()
                    .any(|(inner, _)| group_inner_is_relative(inner))
            {
                for d in &contributed {
                    if *d != read && !second_dates.contains(d) {
                        second_dates.push(*d);
                    }
                }
            }
            return Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates,
            });
        }
    };
    // Enter only for the shapes named above: a group
    // outside the trailing run, or the lone-signal
    // families (a content-headed core asserting, a
    // for-occasion core governing) beside any group.
    let extends_core = match &profile {
        CoreProfile::Assert { single_signal, .. } => *single_signal,
        CoreProfile::Governed { for_occasion, .. } => *for_occasion,
    };
    if !has_non_trailing_group && !extends_core {
        return None;
    }
    let entries = all_group_analyses(text, today, anchor);
    let alternatives: Vec<SimpleDate> = {
        let mut found: Vec<SimpleDate> = Vec::new();
        for (_, a) in &entries {
            for d in &a.alternatives {
                if !found.contains(d) {
                    found.push(*d);
                }
            }
        }
        found
    };
    let entry_dates: Vec<(String, Option<SimpleDate>)> = entries
        .iter()
        .map(|(inner, a)| (inner.clone(), a.contributed))
        .collect();
    match profile {
        CoreProfile::Assert {
            read, second_dates, ..
        } => {
            // Round 25 (R25-B, M3 = red team RT25-7):
            // a bracketed bare restatement, standing
            // in the core, of a value a group on the
            // line governs directly is bound by that
            // governance — "(November 1, cancelled)
            // [November 1]" reads nothing, exactly as
            // the plain surface already reads its
            // twin — because the core's assertion is
            // the sibling group's governed date
            // written again in brackets, not a reading
            // of its own. The unbracketed restatement
            // and every other core assertion stand.
            let governed_here = entries
                .iter()
                .any(|(_, a)| a.governed_values.contains(&read));
            if governed_here && core_bracketed_bare_restatement(core, read, today, anchor) {
                let mut named: Vec<SimpleDate> = Vec::new();
                for d in &alternatives {
                    if *d != read && !named.contains(d) {
                        named.push(*d);
                    }
                }
                return Some(DateLineReading {
                    date: None,
                    weekday_mismatch: None,
                    parenthesized_mismatch: None,
                    second_dates: named,
                });
            }
            let superseded = chain_superseded_from_entries(&entry_dates, read, today, anchor);
            let group_dates: Vec<SimpleDate> =
                entries.iter().filter_map(|(_, a)| a.contributed).collect();
            let remaining: Vec<SimpleDate> = group_dates
                .iter()
                .copied()
                .filter(|d| !superseded.contains(d))
                .collect();
            let parenthesized_mismatch = mismatch_from_groups(&remaining, read, false);
            let mut named: Vec<SimpleDate> = second_dates;
            for d in &superseded {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            for d in &alternatives {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates: named,
            })
        }
        CoreProfile::Governed { values, .. } => {
            let surviving: Vec<SimpleDate> = entries
                .iter()
                .filter_map(|(_, a)| {
                    a.contributed
                        .filter(|d| !(a.contributed_bare && values.contains(d)))
                })
                .collect();
            let Some(read) = surviving.last().copied() else {
                let mut named: Vec<SimpleDate> = Vec::new();
                for d in &alternatives {
                    if !named.contains(d) {
                        named.push(*d);
                    }
                }
                return Some(DateLineReading {
                    date: None,
                    weekday_mismatch: None,
                    parenthesized_mismatch: None,
                    second_dates: named,
                });
            };
            let superseded: Vec<SimpleDate> =
                chain_superseded_from_entries(&entry_dates, read, today, anchor)
                    .into_iter()
                    .filter(|d| surviving.contains(d))
                    .collect();
            let remaining: Vec<SimpleDate> = surviving
                .iter()
                .copied()
                .filter(|d| !superseded.contains(d))
                .collect();
            let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
            let mut named: Vec<SimpleDate> = Vec::new();
            for d in &values {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            for d in &alternatives {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            for d in &superseded {
                if *d != read && !named.contains(d) {
                    named.push(*d);
                }
            }
            // Round 19 (R19-10)'s loser naming, as in
            // `governed_core_group_reading`.
            if surviving.len() >= 2
                && entries
                    .iter()
                    .any(|(inner, _)| group_inner_is_relative(inner))
            {
                for d in &surviving {
                    if *d != read && !named.contains(d) {
                        named.push(*d);
                    }
                }
            }
            Some(DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates: named,
            })
        }
    }
}

/// The trailing groups' contributed dates after
/// the core's governance has bound its bare
/// restatements (round 20, R20-1): the list the
/// Keep-through fallback (round 9, R9-9) reads,
/// so a fallback beside a governed core asserts
/// exactly what the unified composition asserts —
/// a surviving group, never a date the core
/// governed and the group only restated bare.
/// Beside any other core the list is the standing
/// trailing-group list unchanged.
///
/// Round 21 (R21-1/R21-2): the fallback reads the
/// WHOLE line through the same composition — a line
/// the whole-line composition reads contributes
/// exactly its read date (or nothing, when the
/// composition bound every group: a for-occasion
/// core's bare restatement group must not become
/// the keep-through date the For Sunday line
/// refused), and beside a core the composition does
/// not reach (a relative in-words core) the group
/// list is every group on the line, leading groups
/// included, against the core the groups leave
/// behind — "(November 8) next Sunday" keeps
/// through November 8 exactly as its trailing twin
/// does.
pub(super) fn trailing_dates_after_core_governance(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    // Round 35 (R35-C, F3): the fallback reads the
    // line through the same composition as the date
    // lines, so it is handed the same re-spaced text
    // (see `date_line`): on the glued month–day text
    // the composition below composed the date the
    // spaced form refuses, and the fallback stood it
    // back up as the keep-through date.
    let respaced = spaced_glued_weekday_garnish(text);
    let text: &str = respaced.as_deref().unwrap_or(text);
    // Round 35 (R35-F, F6): the same stray-mark
    // spacing as the date lines (see `date_line`),
    // for the same reason — the fallback composes
    // the line's groups against its core, and a
    // malformed group's words are bare words there
    // too.
    let stray_spaced = stray_mark_spaced(text);
    let text: &str = stray_spaced.as_deref().unwrap_or(text);
    // Round 41 (R41-D, F5): the re-dress
    // consultation is first taken here, on the
    // line as written past the stray-mark spacing,
    // as in `date_line` (see there) — the mark
    // shed below eats a line-end quoted group's
    // closing quote, and the trailing-alone group
    // must stand as a group when the composition
    // below reads the line. The standing
    // consultation after the sheds still runs when
    // this one does not fire.
    let redressed_early = quoted_groups_redressed_where_judged(text, today, anchor);
    let text: &str = redressed_early.as_deref().unwrap_or(text);
    // Round 37 (R37-F, F9): the same bare-mark shed
    // as the date lines (see `date_line`), for the
    // same reason — the fallback reads the line's
    // bare words too, and a mark the grouped readers
    // shed is garnish there as well.
    let mark_shed = bare_date_mark_shed(text);
    let text: &str = mark_shed.as_deref().unwrap_or(text);
    // Round 39 (R39-F, F8): the same quote shed as
    // the date lines (see `date_line`), for the
    // same reason — the fallback reads the line's
    // bare words too, and a quote-wrapped date word
    // is transparent there as well.
    let quote_shed = bare_date_quote_shed(text);
    let text: &str = quote_shed.as_deref().unwrap_or(text);
    // Round 41 (R41-D, F5): the same re-dress
    // consultation as the date lines (see
    // `date_line`), for the same reason — the
    // fallback composes the line's groups against
    // its core, and a quoted date group standing
    // alone beside a relative core is a group to
    // the composition it feeds: "(November 8,
    // 2026) next Sunday" keeps through November 8
    // here, while its quoted twin, whose group the
    // fallback could not see standing as written,
    // lost the keep-through date entirely.
    let redressed = if redressed_early.is_none() {
        quoted_groups_redressed_where_judged(text, today, anchor)
    } else {
        None
    };
    let text: &str = redressed.as_deref().unwrap_or(text);
    // Round 38 (R38-D, F4): the same leading-governor
    // unwrap as the date lines (see `date_line`), for
    // the same reason — the fallback composes the
    // whole line, and on the wrapped text the
    // composition served the date the bare-governor
    // twin's line governs, standing it back up as
    // the keep-through date the twin never stands.
    let unwrapped = unwrapped_leading_governor_group(text);
    let text: &str = unwrapped.as_deref().unwrap_or(text);
    // Round 39 (R39-H, F10): the same trailing /
    // mid-line governor unwrap as the date lines
    // (see `date_line`), for the same reason — the
    // fallback composes the whole line, and on the
    // wrapped text the composition served the date
    // the bare-governor twin's line governs,
    // standing it back up as the keep-through date
    // the twin never stands.
    let unwrapped_trailing = unwrapped_trailing_or_midline_governor_group(text, today);
    let text: &str = unwrapped_trailing.as_deref().unwrap_or(text);
    if let Some(reading) = whole_line_composition(text, today, anchor) {
        return reading.date.into_iter().collect();
    }
    let spans = balanced_spans(text);
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    let governed_values = every_governed_core_values(core.trim(), today, anchor);
    if governed_values.is_empty() {
        // A leading group joins the fallback only
        // beside a relative core it disagrees with
        // (the R21-1 shape above): beside any other
        // core a group standing before the words is
        // not the fallback's to read — the R10-10c
        // boundary — and the list is the standing
        // trailing-group list.
        let core_words = core.replace(';', " ");
        let core_trim = core_words.trim();
        let after_shape = sunday_after_relative_shape(core_trim);
        if after_shape || relative_sunday_shape(core_trim) {
            let resolution = relative_phrase_resolution(core_trim, today, after_shape);
            if leading_group_disagrees(text, today, resolution) {
                return all_parenthesized_dates(text, today, anchor);
            }
        }
        return trailing_parenthesized_dates(text, today, anchor);
    }
    all_group_analyses(text, today, anchor)
        .iter()
        .filter_map(|(_, a)| {
            a.contributed
                .filter(|d| !(a.contributed_bare && governed_values.contains(d)))
        })
        .collect()
}

/// The base dates a line's Sunday-after phrases
/// consume (round 19, R19-8): every "Sunday after
/// <base>" run standing in the line's core, read
/// with the second-date scan's own run scan
/// (`sunday_after_phrase_run`), so a phrase's base
/// is known wherever the phrase stands in the line.
/// A consumed base is part of its phrase — the
/// phrase's resolution is the date the line names —
/// and is never another date beside it.
pub(super) fn sunday_after_consumed_bases(text: &str, today: SimpleDate) -> Vec<SimpleDate> {
    let core = strip_all_trailing_parenthesized(text.trim());
    let tokens: Vec<&str> = core.split_whitespace().collect();
    let cleaned: Vec<String> = tokens.iter().map(|t| group_token(t)).collect();
    let mut bases: Vec<SimpleDate> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some((base, end)) = sunday_after_phrase_run(&tokens, &cleaned, i, today) {
            if !bases.contains(&base) {
                bases.push(base);
            }
            i = end.max(i + 1);
            continue;
        }
        i += 1;
    }
    bases
}

/// Whether a parenthesized group's own words carry
/// a relative Sunday phrase (round 19, R19-10): a
/// "next Sunday" / "this Sunday" phrase, or a
/// "Sunday after" phrase — the group signals a
/// relative Sunday, not only a framed date.
pub(super) fn group_inner_is_relative(inner: &str) -> bool {
    let tokens: Vec<String> = inner.split_whitespace().map(group_token).collect();
    tokens.windows(2).any(|w| {
        ((w[0] == "next" || w[0] == "this") && w[1] == "sunday")
            || (w[0] == "sunday" && w[1] == "after")
    })
}

/// The weekday mismatch a plain segment of the
/// whole-line composition carries, if one does
/// (round 22, R22-3): the line's de-grouped core —
/// every group taken out, a semicolon standing where
/// each group stood, as `whole_line_composition`
/// builds it — is read segment by segment with the
/// standing inner reader, and the first segment
/// whose framed date disagrees with its weekday
/// answers. A segment standing at the start of the
/// core (directly after a leading group) frames its
/// date there exactly as the same words frame it at
/// the start of a line, and a weekday wearing "on"
/// frames wherever it stands, so no segment position
/// exempts a framed occurrence from the check. A
/// line with no group has no composition segments
/// and is never judged here.
pub(super) fn composition_segment_weekday_mismatch(
    text: &str,
    today: SimpleDate,
) -> Option<WeekdayMismatch> {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return None;
    }
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    for segment in core.split(';') {
        // Round 23 (R23-3): the separator standing where
        // the group was taken off — comma, colon, dash
        // (incl. en/em), period, slash, or the word
        // "and" — is a boundary exactly as the space and
        // semicolon already are: it is stripped from the
        // segment's head before the segment is judged, so
        // "(November 8), Monday, November 15, 2026" (and
        // its colon / dash / period / slash / "and"
        // twins) records the same mismatch and asserts
        // nothing, exactly as the space twin does. A
        // segment carrying no word of its own is skipped.
        const SEGMENT_BOUNDARY: [char; 10] = [
            ' ', '\t', ';', ',', ':', '.', '/', '-', '\u{2013}', '\u{2014}',
        ];
        let mut segment = segment.trim_matches(SEGMENT_BOUNDARY);
        // Round 24 (R24-4): a standalone boundary
        // token at the segment's head ("*", "•", "|",
        // "~", "_", an emoji — any token with no
        // letters and no digits) is stripped the same
        // way, before and after the "and" twin, so
        // "(November 1) * Monday November 15, 2026"
        // records its mismatch as its comma twin
        // does.
        segment = strip_leading_boundary_tokens(segment);
        segment = segment.trim_matches(SEGMENT_BOUNDARY);
        if let Some(rest) = segment
            .strip_prefix("and ")
            .or_else(|| segment.strip_prefix("And "))
            .or_else(|| segment.strip_prefix("AND "))
        {
            segment = rest.trim_matches(SEGMENT_BOUNDARY);
            segment = strip_leading_boundary_tokens(segment);
            segment = segment.trim_matches(SEGMENT_BOUNDARY);
        }
        if segment.is_empty() || !segment.chars().any(|c| c.is_alphanumeric()) {
            continue;
        }
        let segment_raw = Some(segment.to_string());
        let (_, mismatch) = date_line_inner(&segment_raw, today);
        if mismatch.is_some() {
            return mismatch;
        }
    }
    None
}

/// `text` with every fully quote-wrapped DATE
/// group re-dressed as its paren twin — the
/// opening quote character replaced by "(" and the
/// closing quote by ")" — when any stand (round
/// 40, R40-D, F6 = red team RT40-B). A quoted span
/// is a group here only when its whole contents
/// parse as a date under the guard's own judgment
/// (`group_date`, the judgment
/// `group_split_sunday_after_phrase` applies to a
/// bracket group's contents): a quoted weekday, a
/// quoted governor, and every other quoted span
/// keep their bytes, so this pass cannot mint a
/// group for any other reader's contents class.
/// The quote family is the standing one — straight
/// "...", curly “...”, and single '...' quotes,
/// mixed as the bare quote shed allows. The scan
/// takes the first closing-family quote after an
/// opening one as the span's end; a span whose
/// contents are no date is stepped over at its
/// closing quote, which is reconsidered as an
/// opener once (a glued neighbour span), and a
/// line whose quotes never close costs one scan —
/// the pass is linear in the line beside the date
/// judgments themselves.
pub(super) fn quoted_date_groups_as_parens(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<String> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    // (open byte start, open byte len, close byte
    // start, close byte len) per re-dressed span.
    let mut spans: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let (start, c) = chars[i];
        if !matches!(c, '"' | '\'' | '\u{201c}' | '\u{2018}') {
            i += 1;
            continue;
        }
        let Some(j) = (i + 1..chars.len())
            .find(|&k| matches!(chars[k].1, '"' | '\'' | '\u{201d}' | '\u{2019}'))
        else {
            break;
        };
        let (close_start, close_c) = chars[j];
        let inner = &text[start + c.len_utf8()..close_start];
        if !inner.trim().is_empty() && group_date(inner, today, anchor).is_some() {
            spans.push((start, c.len_utf8(), close_start, close_c.len_utf8()));
            i = j + 1;
        } else {
            i = j;
        }
    }
    if spans.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0usize;
    for (open, open_len, close, close_len) in &spans {
        out.push_str(&text[at..*open]);
        out.push('(');
        out.push_str(&text[open + open_len..*close]);
        out.push(')');
        at = close + close_len;
    }
    out.push_str(&text[at..]);
    Some(out)
}

/// Whether a line is a group — any of the three
/// wrapper pairs — standing beside a core that is
/// only a relative Sunday phrase (round 36, R36-D,
/// F4; round 37, R37-B, F1). This is the in-words
/// machinery's shape: the R21-1 readings remember
/// the phrase in words and surface the group on the
/// parenthesized record, and the line itself reads
/// no date. The plain multi-date reading must not
/// adopt the group's date as a plain framed signal
/// on such a line — for a paren group it never could
/// (the plain scan cannot read a date through a
/// parenthesis), but the scan's date reader trims
/// brackets and braces at a group edge, so a
/// bracket/brace group was adopted as the line's
/// date and the phrase forgotten. Round 36 judged
/// only a group standing at the line's head; past
/// any leading word ("notes [November 8, 2026] next
/// Sunday") the same adoption stood, field-identical
/// to the bare reading, so the judgment is consulted
/// wherever the group stands before the phrase, in
/// this one place. The check is the shape itself:
/// a balanced group stands before the phrase, the
/// de-grouped core (built as `whole_line_composition`
/// builds it, a semicolon where each group stood)
/// carries no framed plain signal — a framed date
/// beside the phrase is the plain composition's
/// shape, not this one — and the words from the
/// group onward are only the relative phrase (at
/// the line's head, the whole core is the phrase;
/// past a prefix, the phrase stands alone after
/// the last group before it, and no signal of any
/// kind stands in the words before it). One
/// witnessed position is excepted, exactly as the
/// red team's round-37 sweep held it: two or more
/// groups standing before the phrase that ALL carry
/// the same date (the stacked agreeing groups) keep
/// their standing reading — that date is adopted in
/// every pair form — so the judgment does not fire
/// there.
/// Round 38 (R38-C, F3): two further positions take
/// the same judgment. A group standing after a
/// weekday word before the phrase ("Sunday
/// [November 8, 2026] next Sunday") escaped the
/// non-leading check because the weekday word is
/// the Sunday the phrase names: its one plain
/// signal is the bare-Sunday signal the word itself
/// spawns, the phrase's own material, not a signal
/// standing beside the phrase — so a prefix that is
/// only that word (with its "on"/"the" framing
/// stack) does not disarm the judgment, and the
/// stacked agreeing exception, whose adopting
/// stacks stand at the signal-free prefixes, does
/// not extend there (at this slot the paren stacks
/// are silent, agreeing, disagreeing, and mixed
/// alike).
/// Round 39 (R39-B, F3): the after-weekday-word arm
/// takes ANY weekday-word prefix under the Sunday
/// arm's reading — a prefix that is only one
/// weekday word (with its "on"/"the" stack) does
/// not disarm the judgment whatever weekday it
/// names, and the stacked agreeing exception does
/// not extend to a weekday-word prefix either (a
/// Monday prefix spawns no plain signal, so the
/// exception's signal-free gate alone would have
/// let an agreeing stack adopt after a Monday
/// where the paren stacks are silent, exactly as
/// after a Sunday). And the split-phrase judgment
/// below is consulted BEFORE the framed-signal
/// bail: at the positions round 39 adds — a group
/// standing between "after" and the base, a group
/// standing between a weekday word and "after"
/// before an absolute base — the de-grouped core's
/// one framed signal is the phrase's own base,
/// which the core scan cannot consume as a phrase
/// base across the group marker, so the bail would
/// disarm the judgment on the phrase's own
/// material, as the Sunday prefix's signal once
/// did. And a date group standing between a
/// Sunday-after phrase's own words ("Sunday
/// [November 8, 2026] after November 1, 2026", the
/// phrase's lead-in words and all) is judged by
/// `group_split_sunday_after_phrase` below: the
/// phrase there is absolute, not relative, so it is
/// a second shape beside this one's, consulted in
/// this same place.
pub(super) fn group_beside_relative_core(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return false;
    }
    // Round 39 (R39-B, F3): the split-phrase
    // judgment is consulted first (see the head
    // comment): its shapes are exact, and at the
    // between-"after"-and-base position the core
    // scan below would bail on the phrase's own
    // base standing framed.
    if group_split_sunday_after_phrase(text, &spans, today, anchor) {
        return true;
    }
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    let core_words = core.replace(';', " ");
    let core_trim = core_words.trim();
    let tokens: Vec<&str> = core_trim.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&tokens);
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    if signals.iter().any(|s| s.framed) {
        return false;
    }
    // The leading position (round 36): the whole
    // de-grouped core is only the relative phrase.
    let trimmed = text.trim_start();
    let lead = text.len() - trimmed.len();
    if spans.first().map(|(start, _)| *start) == Some(lead) {
        return relative_sunday_shape(core_trim) || sunday_after_relative_shape(core_trim);
    }
    // The non-leading positions (round 37): the
    // line's segments are the text between its
    // groups, in written order. The phrase segment
    // is the last segment from which the rest of
    // the line is only the relative phrase; every
    // group before it stands before the phrase.
    let mut segments: Vec<&str> = Vec::with_capacity(spans.len() + 1);
    let mut at = 0usize;
    for (start, end) in &spans {
        segments.push(&text[at..*start]);
        at = *end;
    }
    segments.push(&text[at..]);
    // Round 39 (R39-G, F9/F11): the suffix judgments
    // read a word list maintained as the walk
    // descends, not a rejoined suffix string per
    // position. The join this replaces rebuilt the
    // whole remaining suffix — and the shape readers
    // re-tokenized and re-cleaned it — at every
    // segment boundary, so a line of thousands of
    // groups (whose segments are the empty strings
    // and ";" marks between them) paid quadratic
    // bytes of joining and cleaning for a question
    // about at most a phrase's worth of words. The
    // words are the shapes' own: the text forms of
    // both shapes tokenize on whitespace, clean by
    // `group_token`, and drop the empty words, and
    // the joined suffix's token stream is the
    // segments' token streams in order (the join's
    // space is a separator, as a segment edge is),
    // so the per-segment lists concatenate to
    // exactly the word list the joined string's
    // shapes read. The judgments below are the
    // standing token forms of the same two shapes.
    let segment_words: Vec<Vec<String>> = segments
        .iter()
        .map(|segment| {
            segment
                .split_whitespace()
                .map(group_token)
                .filter(|w| !w.is_empty())
                .collect()
        })
        .collect();
    let mut suffix_words: Vec<String> = Vec::new();
    for p in (1..segments.len()).rev() {
        let mut words: Vec<String> =
            Vec::with_capacity(segment_words[p].len() + suffix_words.len());
        words.extend(segment_words[p].iter().cloned());
        words.extend(suffix_words.iter().cloned());
        suffix_words = words;
        if !relative_sunday_shape_tokens(&suffix_words)
            && !sunday_after_relative_shape_tokens(&suffix_words)
        {
            continue;
        }
        // No signal of any kind stands in the words
        // before the phrase segment: a framed date
        // there is already excluded by the core scan
        // above, and a relative phrase standing
        // before the group is the plain reading's
        // restated-relative shape, not this one.
        // Round 38 (R38-C, F3): the one prefix whose
        // signal is the phrase's own material — the
        // weekday word "Sunday", with its "on"/"the"
        // framing stack — is not a signal standing
        // before the group: the word spawns a
        // bare-Sunday signal by standing there at
        // all, and a group after it ("Sunday
        // [November 8, 2026] next Sunday") is at the
        // phrase's interior, judged as the groups at
        // the signal-free prefixes are.
        // Round 39 (R39-B, F3): the prefix is ANY
        // weekday word, not only "Sunday" — a group
        // after a Monday (or Tuesday, or Wednesday)
        // word before the phrase is at the same
        // interior, judged as the Sunday arm judges
        // it; the other weekday words spawn no plain
        // signal of their own, so for them the gate
        // below is the exception gate beside it.
        let prefix = segments[..p].join(" ");
        let prefix_tokens: Vec<&str> = prefix.split_whitespace().collect();
        let prefix_cleaned: Vec<String> = segment_tokens(&prefix_tokens);
        let prefix_signals = scan_plain_signals(&prefix_tokens, &prefix_cleaned, today, anchor);
        let prefix_words: Vec<&str> = prefix_cleaned
            .iter()
            .map(String::as_str)
            .filter(|w| !w.is_empty())
            .collect();
        let mut word_at = 0usize;
        while matches!(prefix_words.get(word_at), Some(&"on") | Some(&"the")) {
            word_at += 1;
        }
        let weekday_word_prefix =
            prefix_words.len() - word_at == 1 && weekday_word(prefix_words[word_at]).is_some();
        if !prefix_signals.is_empty() && !weekday_word_prefix {
            return false;
        }
        // The stacked agreeing exception: every
        // group standing before the phrase carries
        // the same date (see the head comment). The
        // exception is the signal-free prefixes'
        // standing reading; it does not extend to
        // the weekday-word prefix, where no stack
        // adopts in the paren form (round 38; round
        // 39: at any weekday word, whose prefixes
        // are signal-free but for Sunday's own).
        let groups_before = &spans[..p];
        if prefix_signals.is_empty() && !weekday_word_prefix && groups_before.len() >= 2 {
            let mut dates = groups_before
                .iter()
                .map(|(start, end)| group_date(&text[start + 1..end - 1], today, today));
            if let Some(first) = dates.next() {
                if first.is_some() && dates.all(|d| d == first) {
                    return false;
                }
            }
        }
        return true;
    }
    // The split-phrase position is consulted at
    // the head of this function (round 39; it was
    // consulted here in round 38, past the
    // framed-signal bail its new positions cannot
    // pass).
    false
}

/// Whether a date group stands between a Sunday-after
/// phrase's own words in `text` (round 38, R38-C, F3):
/// the phrase's "Sunday" and "after" are consecutive
/// plain words with a group standing between them, at
/// least one of the groups there carries a date, the
/// words after "after" are exactly the phrase's base
/// (one trailing weekday garnish word aside), no group
/// stands at or after the "after", and the words
/// before the "Sunday" are ordinary lead-in — the
/// framing stack and plain words, never a weekday word
/// or a relative phrase's "next"/"this", whose lines
/// are the standing readers' own shapes (a preceding
/// relative phrase reads twin-equal in every pair, and
/// a framing weekday is the weekday machinery's). A
/// governor or inert group at the split is garnish in
/// every pair already, and a weekday-only group there
/// is read by the phrase's own readers in every pair,
/// so only a date-carrying group is this judgment's:
/// its bracket/brace date is what the plain reading
/// adopted while the paren twin read nothing. At this
/// slot no stack adopts in the paren form — agreeing,
/// disagreeing, and mixed stacks are silent alike — so
/// no stacked exception stands here.
/// Round 39 (R39-B, F3): the judgment extends to the
/// split's further positions, and the plain words are
/// read off the text BETWEEN the balanced spans (a
/// segment tokenization), so a group glued against
/// the phrase's first word stands between the same
/// two words as its spaced twin — "Sunday[November 8,
/// 2026] after ..." is this slot's, while a group
/// glued against the "after" ("...]after ...") is
/// not: that dress already reads as its twins do.
/// The further positions: (1) the phrase's first
/// word may be ANY weekday word before an absolute
/// base ("Monday [November 8, 2026] after November
/// 1, 2026" — the paren twin serves the base with
/// the group recorded, the Sunday form's reading);
/// (2) a framing word may stand after "after" before
/// the base ("Sunday [November 8, 2026] after the
/// November 1, 2026" — the "on"/"the" stack the
/// base itself may wear); (3) a date group may stand
/// between "after" and the base of a Sunday-after
/// phrase ("Sunday after [November 8, 2026]
/// November 1, 2026" — the group is the first thing
/// after "after"; a framing word standing between
/// "after" and the group, "after the [..]", already
/// reads as its paren twin does and is not this
/// judgment's; framing between the group and the
/// base is the base's own dress, as in (2)). At
/// every position the base is read from the written
/// base and the group is recorded against it, as
/// the paren twin is read.
pub(super) fn group_split_sunday_after_phrase(
    text: &str,
    spans: &[(usize, usize)],
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    // The plain words of the line, in written order,
    // with byte ranges: the words of the text BETWEEN
    // the balanced spans, tokenized segment by
    // segment. Round 39 (R39-B, F3): the round-38
    // tokenization took a whitespace token for a
    // plain word when it started outside a span, so
    // a group glued against the phrase's first word
    // ("Sunday[November 8, 2026] after ...") fused
    // that word into the group's own token and the
    // split position was invisible; on the segments
    // the glued word stands as its own word, exactly
    // as the spaced twin's does.
    // Each entry: (byte start, byte end, raw, cleaned).
    let mut plain: Vec<(usize, usize, &str, String)> = Vec::new();
    let mut seg_start = 0usize;
    for idx in 0..=spans.len() {
        let seg_end = spans.get(idx).map(|(s, _)| *s).unwrap_or(text.len());
        let segment = &text[seg_start..seg_end];
        let mut at: Option<usize> = None;
        for (i, c) in segment.char_indices() {
            if c.is_whitespace() {
                if let Some(s) = at.take() {
                    let raw = &segment[s..i];
                    let cleaned = group_token(raw);
                    if !cleaned.is_empty() {
                        plain.push((seg_start + s, seg_start + i, raw, cleaned));
                    }
                }
            } else if at.is_none() {
                at = Some(i);
            }
        }
        if let Some(s) = at {
            let raw = &segment[s..];
            let cleaned = group_token(raw);
            if !cleaned.is_empty() {
                plain.push((seg_start + s, seg_start + segment.len(), raw, cleaned));
            }
        }
        if let Some((_, e)) = spans.get(idx) {
            seg_start = *e;
        }
    }
    // Whether the plain words from index `from` are
    // exactly a base date, a framing stack ("the" /
    // "on", the dress the base itself may wear)
    // aside: a date prefix consuming them whole, or
    // consuming all but a trailing weekday garnish
    // word.
    let base_consumes = |from: usize| -> bool {
        let mut i = from;
        while plain
            .get(i)
            .is_some_and(|(_, _, _, w)| w.as_str() == "the" || w.as_str() == "on")
        {
            i += 1;
        }
        let rest: Vec<&str> = plain[i..].iter().map(|(_, _, raw, _)| *raw).collect();
        if rest.is_empty() {
            return false;
        }
        let Some((_, used)) = parse_date_prefix(&rest, today) else {
            return false;
        };
        if used == rest.len() {
            return true;
        }
        used + 1 == rest.len() && weekday_word(&group_token(rest[used])).is_some()
    };
    for w in 0..plain.len() {
        // The phrase's first word: a weekday word —
        // "Sunday" itself (round 38), or any weekday
        // word before an absolute base (round 39).
        if weekday_word(&plain[w].3).is_none() {
            continue;
        }
        // The lead-in before the phrase's own word:
        // ordinary words only (see the head comment).
        let lead_in_ok = plain[..w].iter().all(|(_, _, _, word)| {
            weekday_word(word).is_none() && word.as_str() != "next" && word.as_str() != "this"
        });
        if !lead_in_ok {
            continue;
        }
        let Some(after) = plain.get(w + 1) else {
            continue;
        };
        if after.3 != "after" {
            continue;
        }
        let opener_end = plain[w].1;
        let after_start = after.0;
        let after_end = after.1;
        // The split position (round 38; round 39 for
        // the glued and weekday-opener forms): a
        // date-carrying group stands between the
        // phrase's two words, no group stands at or
        // after the "after", and the "after" is not
        // glued against the split group (a group
        // glued against the "after" already reads
        // as its twins do — see the head comment).
        let split_spans: Vec<&(usize, usize)> = spans
            .iter()
            .filter(|(s, e)| *s >= opener_end && *e <= after_start)
            .collect();
        if !split_spans.is_empty()
            && split_spans
                .iter()
                .any(|(s, e)| group_date(&text[s + 1..e - 1], today, anchor).is_some())
            && split_spans.iter().all(|(_, e)| *e < after_start)
            && !spans.iter().any(|(s, _)| *s >= after_start)
            && base_consumes(w + 2)
        {
            return true;
        }
        // The between-"after"-and-base position
        // (round 39): the opener is the phrase's own
        // "Sunday"; a date-carrying group is the
        // first thing after "after" (no plain word
        // stands between "after" and the group — a
        // framing word there already reads as its
        // paren twin does); the words after the
        // group are exactly the base, its framing
        // stack aside; and no group stands at or
        // after the base's first word.
        if plain[w].3 != "sunday" {
            continue;
        }
        let zone: Vec<&(usize, usize)> = spans.iter().filter(|(s, _)| *s >= after_end).collect();
        let Some(first_zone) = zone.first() else {
            continue;
        };
        if plain[w + 2..]
            .iter()
            .take_while(|(s, _, _, _)| *s < first_zone.0)
            .next()
            .is_some()
        {
            continue;
        }
        // The base's first plain word: the first
        // non-framing plain word standing after the
        // zone's groups.
        let Some(base_at) = plain[w + 2..]
            .iter()
            .position(|(s, _, _, cleaned)| {
                *s >= first_zone.1 && cleaned.as_str() != "the" && cleaned.as_str() != "on"
            })
            .map(|i| w + 2 + i)
        else {
            continue;
        };
        let base_start = plain[base_at].0;
        let zone_carries_date = zone
            .iter()
            .filter(|(s, e)| *s >= after_end && *e <= base_start)
            .any(|(s, e)| group_date(&text[s + 1..e - 1], today, anchor).is_some());
        if !zone_carries_date {
            continue;
        }
        if spans.iter().any(|(s, _)| *s >= base_start) {
            continue;
        }
        if base_consumes(base_at) {
            return true;
        }
    }
    false
}

/// Whether a date group stands at one of the two
/// framing positions the whole-line composition
/// judges for bracket groups but the interior-slot
/// guard above does not reach (round 41, R41-C, F4
/// = review R41-R1 + red team RT41-D2). The text
/// judged here is a re-dressing candidate (see
/// `quoted_date_groups_as_parens`): its first group
/// is the candidate group, and it must carry a
/// date under the guard's own judgment
/// (`group_date`), so quoted weekday and governor
/// groups are never this judgment's.
///
/// - The framing position proper: "Sunday after
///   <stack> (November 8, 2026) November 1, 2026" —
///   a framing stack of ordinary words (round 41
///   witnessed "the" / "on" / "choir" / "for" /
///   "at" / "dear", singly and in the doubled
///   stacks "the the" / "on the"; round 42,
///   R42-A, widened the stack to every width and
///   word class the composition itself judges —
///   see the arm's own note) stands between the
///   phrase's "after" and the group, and the words
///   after the group are exactly the phrase's
///   base, its own framing stack aside. The
///   guard's between-"after" position ends at the
///   first intervening word, so the bracket
///   reading here is the whole-line composition's:
///   the base served with the group recorded
///   against it. The group may be glued against
///   the framing word or against the base, as the
///   twins may — the plain words are read off the
///   text between the balanced spans, exactly as
///   `group_split_sunday_after_phrase` reads them,
///   so glue fuses no word.
/// - The framing-before-split position: "Sunday
///   the (November 8, 2026) after November 1,
///   2026" — a stack of words (round 41: exactly
///   one framing word, "the" / "on"; round 42,
///   R42-A, widened to every stack the composition
///   judges there — see the arm's own note) stands
///   between the phrase's "Sunday" and the group,
///   and the group stands between that stack and
///   the phrase's "after", the base exactly after
///   it. The composition reads the bracket form
///   there with the group recorded; the guard's
///   split position requires "Sunday" and "after"
///   as consecutive plain words and never fires.
///
/// No other position is this judgment's: the alone
/// positions (a quoted group leading or trailing a
/// relative phrase, or standing alone after
/// "after") are F5's territory and keep their
/// standing readings, and the positions the guard
/// already covers are the guard's.
pub(super) fn quoted_group_at_framing_position(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    let spans = balanced_spans(text);
    let Some(first_span) = spans.first() else {
        return false;
    };
    if group_date(&text[first_span.0 + 1..first_span.1 - 1], today, anchor).is_none() {
        return false;
    }
    // The plain words of the line, in written
    // order, with byte ranges: the words of the
    // text BETWEEN the balanced spans, tokenized
    // segment by segment, exactly as
    // `group_split_sunday_after_phrase` builds
    // them. Each entry: (byte start, byte end,
    // raw, cleaned).
    let mut plain: Vec<(usize, usize, &str, String)> = Vec::new();
    let mut seg_start = 0usize;
    for idx in 0..=spans.len() {
        let seg_end = spans.get(idx).map(|(s, _)| *s).unwrap_or(text.len());
        let segment = &text[seg_start..seg_end];
        let mut at: Option<usize> = None;
        for (i, c) in segment.char_indices() {
            if c.is_whitespace() {
                if let Some(s) = at.take() {
                    let raw = &segment[s..i];
                    let cleaned = group_token(raw);
                    if !cleaned.is_empty() {
                        plain.push((seg_start + s, seg_start + i, raw, cleaned));
                    }
                }
            } else if at.is_none() {
                at = Some(i);
            }
        }
        if let Some(s) = at {
            let raw = &segment[s..];
            let cleaned = group_token(raw);
            if !cleaned.is_empty() {
                plain.push((seg_start + s, seg_start + segment.len(), raw, cleaned));
            }
        }
        if let Some((_, e)) = spans.get(idx) {
            seg_start = *e;
        }
    }
    // Whether the plain words from index `from`
    // are exactly a base date, a framing stack
    // ("the" / "on", the dress the base itself may
    // wear) aside — the split judgment's own base
    // test, stated the same way.
    let base_consumes = |from: usize| -> bool {
        let mut i = from;
        while plain
            .get(i)
            .is_some_and(|(_, _, _, w)| w.as_str() == "the" || w.as_str() == "on")
        {
            i += 1;
        }
        let rest: Vec<&str> = plain[i..].iter().map(|(_, _, raw, _)| *raw).collect();
        if rest.is_empty() {
            return false;
        }
        let Some((_, used)) = parse_date_prefix(&rest, today) else {
            return false;
        };
        if used == rest.len() {
            return true;
        }
        used + 1 == rest.len() && weekday_word(&group_token(rest[used])).is_some()
    };
    // The framing position proper: the line opens
    // with the phrase's own two words, a framing
    // stack of ordinary words stands between
    // "after" and the group, and the words after
    // the group are exactly the base, no group
    // standing at or after the base's first word.
    // Round 42 (R42-A, F3): the stack is no longer
    // capped at two words — the whole-line
    // composition judges the bracket group at
    // every stack width probed (three, four, five,
    // and six words), and at stacks holding a
    // number token ("after the 3 ...", whose quoted
    // form served the number as a day) or a month
    // word, so the stack words are the alphanumeric
    // words and number tokens at any width. Two
    // exclusions stand, each on the composition's
    // own reading: a weekday word in the stack is
    // not judged in this reading (the paren form
    // there adopts, by its own machinery), and
    // "after" is the phrase's own word, never
    // stack material.
    if plain.len() >= 2 && plain[0].3 == "sunday" && plain[1].3 == "after" {
        let after_end = plain[1].1;
        if first_span.0 >= after_end {
            let framing: Vec<usize> = (2..plain.len())
                .take_while(|&i| plain[i].0 < first_span.0)
                .collect();
            let framing_ok = !framing.is_empty()
                && framing.iter().all(|&i| {
                    let w = plain[i].3.as_str();
                    !w.is_empty()
                        && w.chars().all(|c| c.is_alphanumeric())
                        && weekday_word(w).is_none()
                        && w != "after"
                });
            if framing_ok {
                let base_at = plain.iter().position(|(s, _, _, cleaned)| {
                    *s >= first_span.1 && cleaned.as_str() != "the" && cleaned.as_str() != "on"
                });
                if let Some(base_at) = base_at {
                    if !spans.iter().any(|(s, _)| *s >= plain[base_at].0) && base_consumes(base_at)
                    {
                        return true;
                    }
                }
            }
        }
    }
    // The framing-before-split position: a stack
    // of words stands between "Sunday" and the
    // group, the group stands between the stack
    // and "after", no group stands at or after the
    // "after", and the words after it are exactly
    // the base. Round 41 stated this arm for
    // exactly one framing word ("the" / "on");
    // round 42 (R42-A, F3) widens it to every stack
    // the composition itself judges at this slot —
    // framing-first stacks ("the choir", "on the
    // choir", "the choir hall stage"), ordinary
    // stacks ("choir", "choir hall"), and stacks
    // holding a number token, a month word, a
    // relative word, or a weekday word ("the 3",
    // "the November", "next", "Monday") all carry
    // the bracket reading there, the resolution
    // arm or the base arm as the stack's own shape
    // gives it — so the stack words are the
    // alphanumeric words and number tokens at any
    // width, and the "after" that ends the stack
    // is the first one standing after "Sunday".
    if plain.len() >= 3 && plain[0].3 == "sunday" {
        if let Some(after_at) = (1..plain.len()).find(|&i| plain[i].3 == "after") {
            let stack = &plain[1..after_at];
            let framing_before_ok = !stack.is_empty()
                && stack.iter().all(|(_, end, _, w)| {
                    !w.is_empty() && w.chars().all(|c| c.is_alphanumeric()) && *end <= first_span.0
                })
                && first_span.1 <= plain[after_at].0;
            if framing_before_ok
                && !spans.iter().any(|(s, _)| *s >= plain[after_at].0)
                && base_consumes(after_at + 1)
            {
                return true;
            }
        }
    }
    false
}

/// Whether every group in a line stands ALONE
/// beside a relative Sunday phrase core — each
/// group before the phrase's first word or after
/// its last word, the de-grouped core exactly a
/// relative Sunday phrase (round 41, R41-D, F5 =
/// review R41-R2 + red team RT41-D3). The text
/// judged here is a re-dressing candidate (see
/// `quoted_date_groups_as_parens`): its quoted
/// date groups stand re-dressed as their paren
/// twins, and this judgment is the alone
/// positions' consultation. The guard
/// (`group_beside_relative_core`) reaches a group
/// standing before the phrase and at the phrase's
/// interior, but not one standing after it, so a
/// trailing-alone quoted group was never
/// re-dressed: the line served the phrase's
/// resolution with the group's date demoted to a
/// second-date note, where the paren twin reads
/// no date and remembers the group. A group
/// standing BETWEEN the phrase's own words (the
/// split positions) is not alone and is not this
/// judgment's; a line that is only groups has no
/// phrase beside it — its de-grouped core is no
/// relative shape — and is not this judgment's
/// either, so the lone quoted line keeps its
/// standing silence.
pub(super) fn quoted_group_alone_beside_relative_core(text: &str) -> bool {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return false;
    }
    // The de-grouped core, built as the guard
    // builds it: a semicolon where each group
    // stood, read as words.
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    let core_words = core.replace(';', " ");
    let core_trim = core_words.trim();
    if !relative_sunday_shape(core_trim) && !sunday_after_relative_shape(core_trim) {
        return false;
    }
    // The plain words of the line, with byte
    // ranges: the words of the text between the
    // balanced spans, tokenized segment by segment
    // as `group_split_sunday_after_phrase` reads
    // them, so a group glued against the phrase's
    // first or last word fuses no word.
    let mut plain: Vec<(usize, usize)> = Vec::new();
    let mut seg_start = 0usize;
    for idx in 0..=spans.len() {
        let seg_end = spans.get(idx).map(|(s, _)| *s).unwrap_or(text.len());
        let segment = &text[seg_start..seg_end];
        let mut word_at: Option<usize> = None;
        for (i, c) in segment.char_indices() {
            if c.is_whitespace() {
                if let Some(s) = word_at.take() {
                    if !group_token(&segment[s..i]).is_empty() {
                        plain.push((seg_start + s, seg_start + i));
                    }
                }
            } else if word_at.is_none() {
                word_at = Some(i);
            }
        }
        if let Some(s) = word_at {
            if !group_token(&segment[s..]).is_empty() {
                plain.push((seg_start + s, seg_start + segment.len()));
            }
        }
        if let Some((_, e)) = spans.get(idx) {
            seg_start = *e;
        }
    }
    let (Some(first), Some(last)) = (plain.first(), plain.last()) else {
        return false;
    };
    let (first_start, last_end) = (first.0, last.1);
    spans
        .iter()
        .all(|(s, e)| *e <= first_start || *s >= last_end)
}

/// Whether a date group stands at a plain-line
/// interior position the whole-line composition
/// judges for bracket groups (round 42, R42-A, F3
/// arm (b) = red team RT42-Q1(b)): the candidate
/// line is an ordinary date line — groups aside,
/// its words carry exactly one framed plain date
/// and no Sunday-after phrase (a plain word
/// "after" anywhere in the de-grouped core takes
/// the line out of this judgment: the framing,
/// split, and interior-company positions are the
/// other consultations', and their standing
/// readings do not move). At every such position
/// the bracket twins are judged — the group before
/// the plain date, after it, or with ordinary words
/// around it, and with a governor group standing
/// adjacent in either direction — so the quoted
/// form is re-dressed there and takes the twin's
/// reading outright. A line whose core carries no
/// framed plain date (the alone positions, the
/// split-relative adoption) or more than one is
/// not this judgment's.
pub(super) fn quoted_group_on_plain_date_line(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    let spans = balanced_spans(text);
    if spans.is_empty() {
        return false;
    }
    // The de-grouped core, built as the guard
    // builds it: a semicolon where each group
    // stood, read as words.
    let mut core = String::with_capacity(text.len());
    let mut at = 0usize;
    for (start, end) in &spans {
        core.push_str(&text[at..*start]);
        core.push(';');
        at = *end;
    }
    core.push_str(&text[at..]);
    let core_words = core.replace(';', " ");
    let core_trim = core_words.trim();
    let tokens: Vec<&str> = core_trim.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&tokens);
    if cleaned.iter().any(|w| w == "after") {
        return false;
    }
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    signals.iter().filter(|s| s.framed).count() == 1
}

/// `text` with every DOUBLED quote-wrapped DATE
/// group re-dressed as its paren twin — the doubled
/// opening quotes replaced by "(" and the doubled
/// closing quotes by ")", with no quote byte left
/// standing (round 42, R42-A, F4 = red team
/// RT42-Q2). The single-quote scan
/// (`quoted_date_groups_as_parens`) reads a doubled
/// form as a group wearing a stray quote on each
/// side, and the stray quotes break the alone
/// judgments' shape, so the doubled form was
/// forgotten where its single-quoted twin is
/// remembered. A doubled span is two of the same
/// opening quote (`""`, `''`, `““`), date contents
/// holding no quote byte, and two of the matching
/// closing quote; anything else keeps its bytes.
pub(super) fn doubled_quoted_date_groups_as_parens(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<String> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    // (open byte start, open byte len, close byte
    // start, close byte len) per re-dressed span,
    // the lens covering both quote bytes.
    let mut spans: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut i = 0usize;
    while i + 1 < chars.len() {
        let (start, c) = chars[i];
        let closer = match (c, chars[i + 1].1) {
            ('"', '"') => '"',
            ('\'', '\'') => '\'',
            ('\u{201c}', '\u{201c}') => '\u{201d}',
            _ => {
                i += 1;
                continue;
            }
        };
        let mut close_at = None;
        let mut k = i + 2;
        while k + 1 < chars.len() {
            if chars[k].1 == closer && chars[k + 1].1 == closer {
                close_at = Some(k);
                break;
            }
            k += 1;
        }
        let Some(k) = close_at else {
            break;
        };
        let (close_start, _) = chars[k];
        let open_len = 2 * c.len_utf8();
        let inner = &text[start + open_len..close_start];
        if !inner.trim().is_empty()
            && !inner.chars().any(|ch| {
                matches!(
                    ch,
                    '"' | '\'' | '\u{201c}' | '\u{201d}' | '\u{2018}' | '\u{2019}'
                )
            })
            && group_date(inner, today, anchor).is_some()
        {
            spans.push((start, open_len, close_start, 2 * closer.len_utf8()));
            i = k + 2;
        } else {
            i += 1;
        }
    }
    if spans.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0usize;
    for (open, open_len, close, close_len) in &spans {
        out.push_str(&text[at..*open]);
        out.push('(');
        out.push_str(&text[open + open_len..*close]);
        out.push(')');
        at = close + close_len;
    }
    out.push_str(&text[at..]);
    Some(out)
}

/// The line with its quoted DATE groups re-dressed
/// as their paren twins, where a standing
/// consultation judges a bracket group on the
/// re-dressed line (round 40, R40-D; round 41,
/// R41-C / R41-D): the interior-slot guard
/// (`group_beside_relative_core`) on the candidate
/// or its trailing-stripped core, the framing
/// positions (`quoted_group_at_framing_position`),
/// the alone positions
/// (`quoted_group_alone_beside_relative_core`), or
/// the plain-line interior positions
/// (`quoted_group_on_plain_date_line`). A doubled
/// quote form (`""November 8, 2026""`) is consulted
/// only at the alone positions (round 42, R42-A,
/// F4): at the framing and interior positions the
/// doubled forms split nothing and are not moved.
/// Every reader that must take a bracket twin's
/// reading of a quoted group consults the line
/// through this one door — `date_line`, the Keep
/// route's group fallback, and the memory readers
/// — so a quoted group is judged in the same
/// places by each of them, and nowhere else: a
/// quoted group at a position no consultation
/// covers keeps its standing plain reading byte
/// for byte, and quoted weekday and governor
/// groups (never re-dressed) are untouched.
pub(super) fn quoted_groups_redressed_where_judged(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Option<String> {
    // The doubled candidate is consulted FIRST,
    // and only at the alone positions: the
    // single-quote scan also returns a candidate
    // for a doubled form — the group wearing a
    // stray quote on each side — and that candidate
    // passes the alone judgment's de-grouped shape
    // while its stray quotes break the shape
    // readers the memory machinery judges by, so
    // consulting it first would hand those readers
    // the broken candidate and the doubled form
    // would stay forgotten.
    if let Some(doubled) = doubled_quoted_date_groups_as_parens(text, today, anchor) {
        if quoted_group_alone_beside_relative_core(&doubled) {
            return Some(doubled);
        }
    }
    if let Some(candidate) = quoted_date_groups_as_parens(text, today, anchor) {
        if group_beside_relative_core(&candidate, today, anchor)
            || group_beside_relative_core(
                strip_all_trailing_parenthesized(&candidate),
                today,
                anchor,
            )
            || quoted_group_at_framing_position(&candidate, today, anchor)
            || quoted_group_alone_beside_relative_core(&candidate)
            || quoted_group_on_plain_date_line(&candidate, today, anchor)
        {
            return Some(candidate);
        }
    }
    None
}

/// Read one date line with its round-9 records: the
/// plain readings are `date_line_inner`'s, and the
/// trailing parenthesized dates compose around them.
///
/// - A line the inner reader reads keeps its date; a
///   trailing parenthesized date that disagrees with
///   it records the garnish mismatch (on a
///   Sunday-after line the comparison is with the
///   resolution, the date the line names).
/// - A plain date line the inner reader cannot read
///   at all — "October 25, 2026 (November 1)", whose
///   parenthesis used to poison the whole reading —
///   is re-read on its core, trailing parenthesized
///   groups off, and the same comparison runs. The
///   parenthesis never silently kills the plain
///   reading.
/// - A line that is only parenthesized groups reads
///   the last group's date (round 10, R10-10a), and a
///   group standing between a month-day and its year
///   is compared as garnish without poisoning the
///   plain date (R10-10b).
/// - Every distinct second absolute date in the
///   line, before the read date or after it, is
///   noted, never substituted (round 10, R10-4).
/// - A plain line naming two or more framed dates
///   and reading in no other way reads its first
///   date, the rest noted (round 12, R12-6);
///   NoSundayNamed is reserved for lines naming no
///   date.
pub(super) fn date_line(
    raw: &Option<String>,
    today: SimpleDate,
    anchor: SimpleDate,
) -> DateLineReading {
    // Round 35 (R35-C, F3): the whole line is read on
    // its re-spaced text, not only the inner readers
    // — a glued weekday group at the month–day slot
    // ("November(Monday) 1, 2026") yields no date and
    // no record to `date_line_inner` in either form,
    // so the spaced reading is decided by the composed
    // readers below, and on the glued text those
    // readers stripped the group as anonymous garnish
    // and read the date the spaced form refuses.
    // Handing every reader the spaced twin makes the
    // glued line's full record the spaced line's, at
    // every slot, exactly as `date_line_inner` already
    // hands its own readers the spaced line.
    let respaced = raw
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(spaced_glued_weekday_garnish);
    let effective_raw: Option<String> = match &respaced {
        Some(spaced) => Some(spaced.clone()),
        None => raw.clone(),
    };
    // Round 40 (R40-E, F9): the leading governor
    // company is judged once here, on the pre-stray
    // text, beside the inner reader's own judgment
    // on the text it is handed below. The two agree
    // wherever the passes between here and there
    // keep the company's groups standing; they part
    // only for a crossed group, which the stray-mark
    // pass below re-dresses as bare words ("[cancelled)
    // (Monday]" becomes "cancelled) Monday] ..."), so
    // the inner reader inherits the bare-words line
    // and serves the date the company condemns. The
    // judgment taken here is restored after the
    // inner reading, under its crossed flag and only
    // where the inner reading served the company's
    // own framed date (see the restore below).
    // Round 41 (R41-B, F3): the judgment now also
    // reaches the company behind ordinary lead-in
    // words, and the restore below takes it under
    // its lead-in flag as well: the passes between
    // here and the inner reading bare the mid-line
    // governor group, leaving the inner reader the
    // bare run whose company it does not claim, so
    // a mid-line company whose inner reading served
    // its framed date — or read nothing in silence,
    // the weekday-first and marked arms — takes the
    // composed outcome judged here.
    let leading_company = effective_raw
        .as_deref()
        .and_then(|text| leading_governor_company_judgment(text, today));
    // Round 35 (R35-F, F6, as adjudicated
    // 2026-10-21): the whole line is likewise read
    // with its stray marks set to spaces (see
    // `stray_mark_spaced`) — a malformed group's
    // words judge as the bare words they are, at
    // every reader below, while walk-closed groups
    // keep their marks and their structural effect.
    let effective_raw: Option<String> = match effective_raw {
        Some(text) => match stray_mark_spaced(&text) {
            Some(spaced) => Some(spaced),
            None => Some(text),
        },
        None => None,
    };
    // Round 41 (R41-D, F5): the re-dress
    // consultation (see the standing consultation
    // below) is first taken HERE, on the line as
    // written past the stray-mark spacing. The mark
    // shed below eats the closing quote of a quoted
    // group standing at the line's end — its
    // year-tail arm peels closing quotes as garnish
    // against a bare year, and a quoted span is no
    // balanced group to that arm — so a quoted
    // group standing alone at the line's end (the
    // trailing-alone position) is no group by the
    // standing consultation point and was never
    // re-dressed. Where the consultation fires
    // here, the re-dressed line reads through the
    // remaining passes exactly as its bracket
    // twin's line does; where it does not fire, the
    // standing consultation below runs as it always
    // has, so a group the sheds themselves complete
    // is consulted there.
    let mut redressed_early = false;
    let effective_raw: Option<String> = match effective_raw {
        Some(text) => match quoted_groups_redressed_where_judged(&text, today, anchor) {
            Some(candidate) => {
                redressed_early = true;
                Some(candidate)
            }
            None => Some(text),
        },
        None => None,
    };
    // Round 37 (R37-F, F9): the whole line is
    // likewise read with a sentence mark glued to a
    // bare month word or day number removed (see
    // `bare_date_mark_shed`) — the grouped readers
    // shed the mark at the word's edge, and the bare
    // path reads as its grouped twin does.
    let effective_raw: Option<String> = match effective_raw {
        Some(text) => match bare_date_mark_shed(&text) {
            Some(shed) => Some(shed),
            None => Some(text),
        },
        None => None,
    };
    // Round 39 (R39-F, F8): the whole line is
    // likewise read with a fully quote-wrapped
    // date word's dress removed (see
    // `bare_date_quote_shed`) — a quoted month or
    // day word at its own slot is the word it
    // names, as the quoted weekday word is since
    // R38-F.
    let effective_raw: Option<String> = match effective_raw {
        Some(text) => match bare_date_quote_shed(&text) {
            Some(shed) => Some(shed),
            None => Some(text),
        },
        None => None,
    };
    // Round 40 (R40-D, F6): the whole line is
    // likewise read with a fully quote-wrapped date
    // group re-dressed as its paren twin, but only
    // where the interior-slot guard
    // (`group_beside_relative_core`) fires on the
    // re-dressed line (or its trailing-stripped
    // core, as the guard is consulted below) — a
    // quoted date group at a position the guard
    // covers is judged exactly as its bracket twin
    // is judged there, same outcome and same
    // records, while a quoted date group standing
    // anywhere else keeps its standing plain
    // reading byte for byte, and quoted weekday
    // and governor groups (not date contents) are
    // never re-dressed at all.
    // Round 41 (R41-C, F4): the consultation
    // extends to the two framing positions the
    // whole-line composition judges for bracket
    // groups but the guard does not reach (see
    // `quoted_group_at_framing_position`): a
    // framing stack between "after" and the group,
    // where the quoted form adopted the group's
    // date and the twins serve the base with the
    // record, and the framing-before-split
    // position, where the quoted form carried
    // second-date notes where the twin carries the
    // record. The re-dressed line is read by the
    // same composition that judges the twins
    // there, so the quoted form takes the twin's
    // outcome and records exactly. No position
    // beyond the witnessed ones is consulted: the
    // alone positions keep their standing
    // readings.
    // Round 41 (R41-D, F5): the consultation
    // extends to the alone positions (see
    // `quoted_group_alone_beside_relative_core`),
    // and the consultation itself is the shared
    // one (`quoted_groups_redressed_where_judged`)
    // the memory readers and the Keep route's
    // group fallback consult through the same
    // door. At the trailing-alone position the
    // quoted form served the phrase's resolution
    // with its date demoted to a second-date note;
    // it now reads no date, exactly as its twins
    // read — the consultation for it is the early
    // one above, taken before the mark shed eats
    // its group's closing quote. At the
    // leading-alone position the guard's leading
    // arm already re-dressed the line for its date
    // outcome — this batch's leading-alone work is
    // in the memory readers and the fallback,
    // which never consulted the re-dress at all.
    let effective_raw: Option<String> = match effective_raw {
        Some(text) if !redressed_early => {
            match quoted_groups_redressed_where_judged(&text, today, anchor) {
                Some(candidate) => Some(candidate),
                None => Some(text),
            }
        }
        other => other,
    };
    // Round 38 (R38-D, F4): the whole line is
    // likewise read with a leading governor group
    // unwrapped (see
    // `unwrapped_leading_governor_group`) — a leading
    // group's governor contents are judged as the
    // bare governor is judged, at the leading slot,
    // in every wrapper, so the wrapped line's full
    // record is its bare-governor twin's. The pass
    // walks only the leading run: trailing and
    // base-trailing groups keep their standing
    // readings (coordinator ruling, r38d-ruling.md).
    let effective_raw: Option<String> = match effective_raw {
        Some(text) => match unwrapped_leading_governor_group(&text) {
            Some(unwrapped) => Some(unwrapped),
            None => Some(text),
        },
        None => None,
    };
    // Round 39 (R39-H, F10): the whole line is
    // likewise read with a trailing-slot or mid-line
    // governor group unwrapped (see
    // `unwrapped_trailing_or_midline_governor_group`)
    // — governor contents in a group are judged as
    // the bare governor is judged wherever the bare
    // governor is judged: the leading slot (R38-D),
    // the trailing slot, and the date's segment
    // mid-line. The base-trailing slot is uniform
    // garnish in every dress and does not move: the
    // pass never unwraps a group whose nearest
    // preceding plain date is a Sunday-after base.
    let effective_raw: Option<String> = match effective_raw {
        Some(text) => match unwrapped_trailing_or_midline_governor_group(&text, today) {
            Some(unwrapped) => Some(unwrapped),
            None => Some(text),
        },
        None => None,
    };
    let raw: &Option<String> = &effective_raw;
    let (date, weekday_mismatch) = date_line_inner(raw, today);
    let none_reading = DateLineReading {
        date: None,
        weekday_mismatch: None,
        parenthesized_mismatch: None,
        second_dates: Vec::new(),
    };
    let Some(text) = raw.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return none_reading;
    };
    // Round 38 (R38-C, F3): the group judgment
    // (`group_beside_relative_core`) is consulted
    // against the inner reading here, before the
    // early return below — at the two interior
    // positions the extended guard now judges (a
    // date group between a Sunday-after phrase's own
    // words, a date group after a weekday word
    // before a relative phrase) the inner reader's
    // head flexible read adopts the bracket/brace
    // group's date, and the guard's standing call
    // sites, in the inner-None branches' plain-multi
    // readings, are never reached. When the guard
    // fires, the inner date is discarded and the line
    // reads on exactly as its paren twin does. The
    // guard is judged on the line and on its
    // trailing-stripped core alike: beside a trailing
    // group the split position's guard fires only on
    // the core (the split helper requires no group at
    // or after the phrase's "after"). The scope probe
    // (r38c-scope-probe.log) shows the discard is
    // safe unscoped: across the standing pin families
    // the guard fires with an inner date only at the
    // two new positions — at the R36-D / R37-B
    // positions the inner reader never adopts, and
    // every control and deliberately unreached form
    // (the group between "after" and the base, the
    // "after the" form, the weekday-group split)
    // stands with the guard silent — so no standing
    // inner reading coincides with a firing guard.
    // Round 39 (R39-B, F3): at the after-weekday-word
    // position under a non-Sunday weekday word the
    // inner reader adopts nothing — instead it pairs
    // the weekday word with the bracket/brace
    // group's date and records a weekday mismatch
    // the paren twin never records ("Monday
    // [November 8, 2026] next Sunday" refuses with
    // Monday at November 8 where the paren twin
    // serves November 8), and the mismatch return
    // below would speak before the composed readers
    // can read the line as its twin is read. When
    // the guard fires, a mismatch manufactured by
    // the line's own groups (see
    // `mismatch_is_guard_manufactured`) is discarded
    // with the adoption it rode in on; a mismatch
    // naming any date the groups do not carry is the
    // line's own and stands.
    let stripped = strip_all_trailing_parenthesized(text);
    let guard_fires = group_beside_relative_core(text, today, anchor)
        || group_beside_relative_core(stripped, today, anchor);
    let manufactured = guard_fires
        && match &weekday_mismatch {
            Some(mismatch) => {
                mismatch_is_guard_manufactured(text, today, anchor, mismatch)
                    || mismatch_is_guard_manufactured(stripped, today, anchor, mismatch)
            }
            None => false,
        };
    let (date, weekday_mismatch) = if guard_fires && date.is_some() {
        (None, weekday_mismatch)
    } else if manufactured {
        (date, None)
    } else {
        (date, weekday_mismatch)
    };
    // Round 40 (R40-E, F9): the crossed-company
    // restore (see the judgment taken at the head of
    // this function). A crossed leading governor
    // company whose inner reading served the very
    // date the company frames takes the composed
    // outcome judged on the pre-stray text —
    // governed, with the weekday record where the
    // company's weekday disagrees — instead of the
    // bare-words reading the stray-mark pass left
    // behind. A refused inner reading already
    // stands, and a company with no crossed group
    // was the inner reader's own to compose.
    // Round 41 (R41-B, F3): the same restore for a
    // company standing behind ordinary lead-in
    // words. Its inner reading either served the
    // very date the company frames (governor-first
    // orders, the serving mark dresses) or read
    // nothing in silence (weekday-first orders, the
    // collapsing mark dresses, the third-group arm)
    // — both take the composed outcome judged on
    // the pre-unwrap text. An inner reading that
    // refused with a record, or served any other
    // date, already stands.
    if let Some((framed, mismatch, crossed, behind_lead_in)) = &leading_company {
        if (*crossed || *behind_lead_in)
            && (date == Some(*framed) || (date.is_none() && weekday_mismatch.is_none()))
        {
            return DateLineReading {
                date: None,
                weekday_mismatch: mismatch.clone(),
                parenthesized_mismatch: None,
                second_dates: Vec::new(),
            };
        }
    }
    // Round 11 (R11-6): the alternatives a trailing
    // group offers instead of contributing are named
    // on the second-date note, beside whatever the
    // line itself reads. Round 40 (R40-C, F2): the
    // groups and the alternatives are read off the
    // one trailing-analyses computation (see
    // `trailing_dates_and_alternatives`).
    let (groups, alternatives) = trailing_dates_and_alternatives(text, today, anchor);
    // Round 22 (R22-3): a wrong-weekday framed date
    // standing in a plain segment of the composition
    // speaks first, exactly as it does without the
    // group: "(November 8) Monday, November 15, 2026"
    // yields no date and records the mismatch, where
    // the line used to assert the date because the
    // inner reader treated the weekday as ordinary
    // text standing after the group. The check runs
    // before every composed reading below; a line
    // whose own inner reader already recorded a
    // mismatch records the same one here (its first
    // segment is the line's own opening words).
    if let Some(mismatch) = composition_segment_weekday_mismatch(text, today) {
        return DateLineReading {
            date: None,
            weekday_mismatch: Some(mismatch),
            parenthesized_mismatch: None,
            second_dates: Vec::new(),
        };
    }
    if let Some(read) = date {
        // Round 18 (R18-11): chain links among the
        // trailing groups are named as superseded
        // targets, not compared as disagreeing
        // garnish, when the line's own date is the
        // chain's last target.
        let superseded = chain_superseded_dates(text, today, anchor, read);
        let remaining: Vec<SimpleDate> = groups
            .iter()
            .copied()
            .filter(|d| !superseded.contains(d))
            .collect();
        let parenthesized_mismatch = mismatch_from_groups(&remaining, read, false);
        let mut second_dates = second_dates_in_line(text, today, read);
        for d in &superseded {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        // Round 18 (R18-7): a line the inner readers
        // salvage never used to reach the governor
        // composition, so a governed RELATIVE signal
        // standing before the salvaged date was never
        // named — "cancelled, next Sunday, November 1,
        // 2026" asserted November 1 with October 25
        // nowhere. When the plain composition on the
        // line's core reads the SAME date the inner
        // readers read, its naming joins the note:
        // governed signals are named by their resolved
        // date, as framed ones are. Only the
        // COMPOSITION's naming is taken (never the
        // first-signal fallback's, whose second dates
        // the standing second-date scan already
        // judges — a Sunday-after line's restated
        // base stays a restatement, not "another
        // date"), and a composition reading any other
        // date (or none) changes nothing here — the
        // salvaged reading stands exactly as it stood.
        let core = strip_all_trailing_parenthesized(text);
        if !core.trim().is_empty() {
            if let Some(plain) = plain_composition_reading(core, today, anchor, &alternatives) {
                if plain.date == Some(read) {
                    // Round 19 (R19-8): the merge never
                    // names a Sunday-after phrase's
                    // consumed base. The composition's
                    // signal scan splits a restated base
                    // into its own signal, so "the Sunday
                    // after November 1, 2026, November 1,
                    // 2026, cancelled" named November 1 —
                    // the phrase's own base, restated and
                    // governed — beside the November 8 the
                    // phrase resolves to. The standing
                    // second-date scan already excludes a
                    // consumed base; the merge does too.
                    // The salvaged reading itself (what
                    // asserts) is unchanged.
                    let consumed_bases = sunday_after_consumed_bases(core, today);
                    for d in &plain.second_dates {
                        if *d != read && !consumed_bases.contains(d) && !second_dates.contains(d) {
                            second_dates.push(*d);
                        }
                    }
                }
            }
        }
        for d in &alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        return DateLineReading {
            date: Some(read),
            weekday_mismatch,
            parenthesized_mismatch,
            second_dates,
        };
    }
    if weekday_mismatch.is_some() {
        return DateLineReading {
            date: None,
            weekday_mismatch,
            parenthesized_mismatch: None,
            second_dates: Vec::new(),
        };
    }
    // Round 39 (R39-F, F8): a group standing at the
    // month slot holding only a month word is the
    // month (see `month_group_reading`), spliced as
    // the day group below is spliced at its slot.
    // The consultation stands BEFORE the whole-line
    // composition, not beside the mid-date splice
    // after it: a month-only group contributes no
    // date of its own, so the composition's
    // groups-only branch answers a line holding one
    // with a no-date reading and the splice would
    // never be reached — the mid-date shape needs
    // no such placement because the composition's
    // own mid-date guard returns None for it.
    if let Some(reading) = month_group_reading(text, today, anchor, &groups, &alternatives) {
        return reading;
    }
    // Round 21 (R21-1/R21-2/R21-3): the whole-line
    // composition — a group standing anywhere but
    // the trailing run, or a content-headed /
    // for-occasion core beside its groups — is read
    // here, after the standing weekday-mismatch
    // return (a wrong weekday speaks first, as it
    // always has) and before the standing readings
    // below, which the composition leaves untouched
    // for every other shape (it returns None there).
    if let Some(reading) = whole_line_composition(text, today, anchor) {
        return reading;
    }
    // Round 10 (R10-10b): a group standing between a
    // month-day and its year does not poison the plain
    // date; the group is compared as garnish.
    if let Some(reading) = mid_date_group_reading(text, today, anchor, &groups, &alternatives) {
        return reading;
    }

    // Round 11 (R11-6): the core re-read below no
    // longer waits for a group to have contributed.
    // A group that contributes nothing — several
    // dates, a negated date, no date, nested groups —
    // behaves as if absent: the line's own reading
    // proceeds on its core either way.
    let core = strip_all_trailing_parenthesized(text);
    if core.trim().is_empty() {
        if groups.is_empty() {
            // Only non-contributing groups stand: no
            // date governs, and the alternatives they
            // offered are named on the note.
            return DateLineReading {
                date: None,
                weekday_mismatch: None,
                parenthesized_mismatch: None,
                second_dates: alternatives,
            };
        }
        // Round 10 (R10-10a): a line that is only
        // parenthesized groups reads its date exactly
        // as the Keep-through line has since round 9 —
        // the last group stands, and an earlier group
        // naming a different date is recorded against
        // it, its source stated as the parentheses.
        // Round 15 (R15-6): a date the standing group
        // governed only by its trailing phrase is
        // named on the second-date note, like the
        // alternatives on every other path.
        let read = *groups.last().expect("a group stands");
        // Round 18 (R18-11): a target chain split
        // across the groups names its superseded
        // targets on the note, as the single-group
        // chain does, and those targets are chain
        // links, not disagreeing garnish — they leave
        // the mismatch comparison.
        let superseded = chain_superseded_dates(text, today, anchor, read);
        let remaining: Vec<SimpleDate> = groups
            .iter()
            .copied()
            .filter(|d| !superseded.contains(d))
            .collect();
        let parenthesized_mismatch = mismatch_from_groups(&remaining, read, true);
        let mut second_dates: Vec<SimpleDate> = Vec::new();
        for d in &alternatives {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        for d in &superseded {
            if *d != read && !second_dates.contains(d) {
                second_dates.push(*d);
            }
        }
        // Round 19 (R19-10): a two-group reading
        // mixing a framed group and a relative group
        // names its loser on the note, as a plain
        // mixed line names its other signal —
        // "(November 8) (next Sunday)" asserts
        // October 25 (the last group stands) AND
        // names November 8. Before this, the loser
        // stood only on the mismatch record; the
        // note itself carried nothing, on either
        // line's mirror.
        // Round 20 (R20-9): the rule generalizes —
        // in a multi-group reading with at least one
        // relative group, the losers are named on
        // the note the same way ("(November 8)
        // (next Sunday) (November 15)" names
        // November 8 and October 25).
        // The bare framed stack is not this shape:
        // its loser keeps the standing mismatch-only
        // naming the r18b chain pin has.
        if groups.len() >= 2
            && trailing_group_analyses(text, today, anchor)
                .iter()
                .any(|(inner, _)| group_inner_is_relative(inner))
        {
            for d in &groups {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
        }
        return DateLineReading {
            date: Some(read),
            weekday_mismatch: None,
            parenthesized_mismatch,
            second_dates,
        };
    }
    if core == text {
        // Round 12 (R12-6): a plain line naming two
        // framed dates reads its first, the rest
        // noted — it does not read as naming none.
        // Round 36 (R36-D, F4): a leading group
        // beside a relative-phrase core is excepted —
        // the multi-date reading would adopt a
        // bracket/brace group's date as a plain
        // framed signal and forget the phrase, where
        // the paren twin reads no date and the phrase
        // is remembered in words (see
        // `group_beside_relative_core`). Round 37
        // (R37-B, F1): the exception covers the group
        // at every pre-phrase position, not only the
        // line's head.
        if !group_beside_relative_core(core, today, anchor) {
            if let Some(reading) = plain_multi_date_reading(core, today, anchor, &alternatives) {
                return reading;
            }
        }
        return DateLineReading {
            date: None,
            weekday_mismatch,
            parenthesized_mismatch: None,
            second_dates: alternatives,
        };
    }
    let core_raw = Some(core.to_string());
    let (core_date, core_mismatch) = date_line_inner(&core_raw, today);
    // Round 38 (R38-C, F3): the same consultation on
    // the core re-read — beside a trailing group the
    // core is an interior-position line in its own
    // right ("Sunday [November 8, 2026] after
    // November 1, 2026" stands under its trailing
    // group), and its inner read adopts the group's
    // date the same way, so a firing guard on the
    // core discards that date too and the line falls
    // to the paths below exactly as its paren twin
    // does (whose core inner read is empty there).
    let core_date = if core_date.is_some() && group_beside_relative_core(core, today, anchor) {
        None
    } else {
        core_date
    };
    // Round 39 (R39-B, F3): the same discard for a
    // guard-manufactured mismatch on the core
    // re-read — beside a trailing group the core is
    // the after-weekday-word line in its own right,
    // and its manufactured mismatch would otherwise
    // ride the paths below where its paren twin's
    // core records none.
    let core_mismatch = match &core_mismatch {
        Some(mismatch)
            if group_beside_relative_core(core, today, anchor)
                && mismatch_is_guard_manufactured(core, today, anchor, mismatch) =>
        {
            None
        }
        _ => core_mismatch,
    };
    match core_date {
        Some(read) => {
            let parenthesized_mismatch = mismatch_from_groups(&groups, read, false);
            let mut second_dates = second_dates_in_line(core, today, read);
            for d in &alternatives {
                if *d != read && !second_dates.contains(d) {
                    second_dates.push(*d);
                }
            }
            DateLineReading {
                date: Some(read),
                weekday_mismatch: None,
                parenthesized_mismatch,
                second_dates,
            }
        }
        None => {
            // Round 20 (R20-1): one composition for
            // an every-governed core and its trailing
            // groups — the core's governed values
            // bind bare trailing restatements, and a
            // surviving group asserts here instead
            // of dropping unnamed (see
            // `governed_core_group_reading`).
            if let Some(reading) = governed_core_group_reading(
                text,
                core,
                today,
                anchor,
                &alternatives,
                core_mismatch.clone(),
            ) {
                return reading;
            }
            // Round 12 (R12-6): the same first-date
            // primacy on the de-grouped core, with
            // the trailing groups still compared as
            // garnish against the date it reads.
            // Round 36 (R36-D, F4): the same group
            // exception as on the whole line above —
            // the core here keeps its pre-phrase
            // groups (only the trailing groups were
            // stripped), so a bracket/brace group
            // beside the core's relative phrase is
            // not adopted here either; the line
            // falls to the relative path below, as
            // its paren twin does. Round 37 (R37-B,
            // F1): the exception covers the group at
            // every pre-phrase position.
            let plain_multi = if group_beside_relative_core(core, today, anchor) {
                None
            } else {
                plain_multi_date_reading(core, today, anchor, &alternatives)
            };
            if let Some(mut reading) = plain_multi {
                // Round 16 (R16-4): the composition can
                // read no date at all (every signal
                // governed); then there is no read date
                // for the garnish to disagree with.
                if let Some(read) = reading.date {
                    reading.parenthesized_mismatch = mismatch_from_groups(&groups, read, false);
                }
                return reading;
            }
            // Round 18 (R18-11): the relative path —
            // the core is a relative phrase and the
            // groups carry the chain — names a split
            // chain's superseded targets on the note
            // too, against the last group's date,
            // which the verdict machinery reads as the
            // phrase's parenthesized date.
            let mut second_dates = alternatives.clone();
            if let Some(chain_read) = trailing_parenthesized_date(text, today, anchor) {
                for d in chain_superseded_dates(text, today, anchor, chain_read) {
                    if d != chain_read && !second_dates.contains(&d) {
                        second_dates.push(d);
                    }
                }
            }
            DateLineReading {
                date: None,
                weekday_mismatch: core_mismatch,
                parenthesized_mismatch: None,
                second_dates,
            }
        }
    }
}

/// Whether a weekday mismatch recorded on `text`
/// was manufactured by the text's own groups
/// (round 39, R39-B, F3): every date a group on
/// the line carries is the mismatch's date. At
/// the after-weekday-word position the inner
/// reader pairs the prefix weekday word with a
/// bracket/brace group's adopted date and records
/// the pair as a mismatch where the paren twin
/// records nothing — the group's date is the
/// mismatch's whole substance. A mismatch naming
/// a date no group carries, or standing beside a
/// group carrying a second date, is the line's
/// own and is not this judgment's.
fn mismatch_is_guard_manufactured(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    mismatch: &WeekdayMismatch,
) -> bool {
    let dates: Vec<SimpleDate> = balanced_spans(text)
        .iter()
        .filter_map(|(start, end)| group_date(&text[start + 1..end - 1], today, anchor))
        .collect();
    !dates.is_empty() && dates.iter().all(|d| *d == mismatch.date)
}
