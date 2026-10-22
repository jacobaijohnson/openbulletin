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
    let mut inners: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some((before, inner)) = split_last_trailing_group(rest) {
        inners.push(inner.to_string());
        rest = before;
    }
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

/// Every alternative a trailing parenthesized group
/// offers, in written order (round 11, R11-6): a
/// group that offers two or more dates contributes
/// none of them, and these are named on the line's
/// second-date note instead.
pub(super) fn trailing_group_alternatives(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> Vec<SimpleDate> {
    let mut found: Vec<SimpleDate> = Vec::new();
    for (_, a) in trailing_group_analyses(text, today, anchor) {
        for d in a.alternatives {
            if !found.contains(&d) {
                found.push(d);
            }
        }
    }
    found
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
    let mut rest = text;
    while let Some(stripped) = strip_trailing_parenthesized(rest) {
        rest = stripped;
    }
    rest
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
    if !relative_shape_matches(text, sunday_after_relative_shape_tokens) {
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
    if matches!(text.trim_start().chars().next(), Some('(' | '[' | '{')) {
        let resolution = relative_phrase_resolution(text, today, true);
        if !leading_group_disagrees(text, today, resolution) {
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
    let mut rest = text;
    while let Some(stripped) =
        strip_trailing_parenthesized(rest).or_else(|| strip_leading_parenthesized(rest))
    {
        rest = stripped;
    }
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
    if !relative_shape_matches(text, relative_sunday_shape_tokens) {
        return None;
    }
    // Round 21 (R21-1), bounded by the R10-10c
    // boundary, as in `sunday_after_relative_phrase`:
    // a leading group restating the phrase's own
    // Sunday stays unread; a leading group naming a
    // different Sunday is remembered with the phrase.
    // Round 36 (R36-D, F4): the leading group may
    // wear any of the three pairs, as there.
    if matches!(text.trim_start().chars().next(), Some('(' | '[' | '{')) {
        let resolution = relative_phrase_resolution(text, today, false);
        if !leading_group_disagrees(text, today, resolution) {
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
    t.trim_matches([
        '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
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
    let trailing = weekday_word(trailing_word).map(|n| (trailing_word, n));
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
        let Some(lead_number) = weekday_word(clean(core[i])) else {
            continue;
        };
        if !weekday_is_framed(&core, i) {
            continue;
        }
        let Some((date, used)) = parse_date_prefix(&core[i + 1..], today) else {
            continue;
        };
        if date.weekday() != lead_number {
            let named = clean(core[i]).to_string();
            return Some((None, Some(WeekdayMismatch { named, date })));
        }
        if i + 1 + used == core.len() {
            if let Some((trailing_word, trailing_number)) = trailing {
                if date.weekday() != trailing_number {
                    let named = trailing_word.to_string();
                    return Some((None, Some(WeekdayMismatch { named, date })));
                }
            }
        }
        return Some((Some(date), None));
    }
    None
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
fn respaced_glued_groups_at_date_slots(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut inserts: Vec<usize> = Vec::new();
    for (start, end) in balanced_spans(text) {
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
        // The month–day slot.
        if word_before_is_month(text, start) && day_number_stands_after(text, end) {
            if left_glued {
                inserts.push(start);
            }
            if end < bytes.len() && bytes[end].is_ascii_digit() {
                inserts.push(end);
            }
            continue;
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
            i += 1;
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
            i += 1;
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
        let peeled = text[..end]
            .trim_end_matches(['.', ',', ';', ':', '!', '?', '"', '\'', '\u{201d}'])
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
        if !token.is_empty() && token.chars().all(|c| c == '*' || c == '|') {
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
        let word = tokens[i].trim_matches([
            '(', ')', '[', ']', '{', '}', ',', '.', ';', ':', '!', '?', '"', '\'', '\u{201d}',
        ]);
        if let Some(named_number) = weekday_word(word) {
            let rest: Vec<&str> = tokens
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, t)| *t)
                .collect();
            if let Some(date) = parse_flexible_date(&rest.join(" "), today) {
                if date.weekday() == named_number {
                    return (Some(date), None);
                }
                return (
                    None,
                    Some(WeekdayMismatch {
                        named: word.to_string(),
                        date,
                    }),
                );
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
        let word = tokens[i].trim_matches(['(', ')', '[', ']', '{', '}', ',', '.']);
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
/// hold. The walk is
/// `balanced_spans`' own, statement for statement,
/// except that what it records is the marks
/// themselves.
pub(super) fn stray_mark_spaced(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
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
            } else if text[i + 1..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric())
            {
                // Round 36 (R36-F, F8 = review R36-8):
                // the stray closer glued to a word's
                // head is spaced for all three pairs,
                // as the paren closer has been since
                // round 35 — "]November" and "}November"
                // wear their word exactly as
                // ")November" does.
                stray.push(i);
            }
        }
    }
    stray.extend(stack.iter().copied().filter(|&open| {
        bytes[open] == b'('
            || (open + 1 < bytes.len()
                && is_group_opener(bytes[open + 1])
                && closed_openers.contains(&(open + 1)))
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
            || (bytes[open] != b'('
                && bytes[..open].iter().enumerate().all(|(j, &b)| {
                    b.is_ascii_whitespace() || (is_group_opener(b) && stack.contains(&j))
                }))
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
pub(super) fn mid_date_group_reading(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
    trailing_groups: &[SimpleDate],
    trailing_alternatives: &[SimpleDate],
) -> Option<DateLineReading> {
    for (start, end) in parenthesized_spans(text) {
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
        let Some(read) = cand_date else {
            continue;
        };
        let inner = &text[start + 1..end - 1];
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
                if cleaned[i] == "sunday"
                    && !(i == last_end && last_framed_sunday)
                    // Round 23 (R23-6): the one
                    // weekday-modifier reading,
                    // verb-form adjective shape
                    // included ("Sunday left
                    // luggage"), as in groups.
                    && !weekday_modifier_at(cleaned, i)
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
    let mut trailing_run = 0usize;
    {
        let mut rest = text;
        while let Some(stripped) = strip_trailing_parenthesized(rest) {
            trailing_run += 1;
            rest = stripped;
        }
    }
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

/// Whether a line is a leading group — any of the
/// three wrapper pairs — standing beside a core
/// that is only a relative Sunday phrase (round 36,
/// R36-D, F4). This is the in-words machinery's
/// shape: the R21-1 readings remember the phrase in
/// words and surface the group on the parenthesized
/// record, and the line itself reads no date. The
/// plain multi-date reading must not adopt the
/// group's date as a plain framed signal on such a
/// line — for a paren group it never could (the
/// plain scan cannot read a date through a
/// parenthesis), but the scan's date reader trims
/// brackets and braces at a group edge, so a
/// bracket/brace leading group was adopted as the
/// line's date and the phrase forgotten. The check
/// is the shape itself: the line opens with a
/// balanced group, the de-grouped core (built as
/// `whole_line_composition` builds it, a semicolon
/// where each group stood) is a relative-phrase
/// core, and no framed plain signal stands in that
/// core — a framed date beside the phrase is the
/// plain composition's shape, not this one.
pub(super) fn leading_group_beside_relative_core(
    text: &str,
    today: SimpleDate,
    anchor: SimpleDate,
) -> bool {
    let trimmed = text.trim_start();
    if !matches!(trimmed.chars().next(), Some('(' | '[' | '{')) {
        return false;
    }
    let spans = balanced_spans(text);
    let lead = text.len() - trimmed.len();
    if spans.first().map(|(start, _)| *start) != Some(lead) {
        return false;
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
    if !relative_sunday_shape(core_trim) && !sunday_after_relative_shape(core_trim) {
        return false;
    }
    let tokens: Vec<&str> = core_trim.split_whitespace().collect();
    let cleaned: Vec<String> = segment_tokens(&tokens);
    let signals = scan_plain_signals(&tokens, &cleaned, today, anchor);
    !signals.iter().any(|s| s.framed)
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
    let groups = trailing_parenthesized_dates(text, today, anchor);
    // Round 11 (R11-6): the alternatives a trailing
    // group offers instead of contributing are named
    // on the second-date note, beside whatever the
    // line itself reads.
    let alternatives = trailing_group_alternatives(text, today, anchor);
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
        // `leading_group_beside_relative_core`).
        if !leading_group_beside_relative_core(core, today, anchor) {
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
            // Round 36 (R36-D, F4): the same leading-
            // group exception as on the whole line
            // above — the core here keeps its leading
            // group (only the trailing groups were
            // stripped), so a bracket/brace leading
            // group beside the core's relative phrase
            // is not adopted here either; the line
            // falls to the relative path below, as
            // its paren twin does.
            let plain_multi = if leading_group_beside_relative_core(core, today, anchor) {
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
