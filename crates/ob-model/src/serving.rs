//! Serving lines and their rename migration: the
//! built-in participant lines (readers, ushers, ...),
//! their permanent slugs, and the machinery that
//! carries stored names across a rename, a reorder,
//! or a slug conflict without crossing one person's
//! names onto another line.

use std::collections::BTreeMap;

/// The built-in serving lines (docs/planning/
/// beyond-print.md section 4, grown by roles build
/// R-2b): who is serving this week, beside the sermon
/// title on the This Sunday step (the J2 pattern). Each
/// is (the line's permanent slug, its built-in display
/// name). A template prints a line's names with the
/// token `{Sunday.<slug>}`. Weekly by nature: Next
/// Sunday clears them and names what it cleared.
///
/// The slug is the line's identity; the display name is
/// the church's words and can change. This table is the
/// fixed alias between the two for the built-ins: token
/// resolution tries the slug of the name as shown AND
/// this table, so a line's token keeps working after
/// the church renames the line (see
/// [`Document::participant_value`]).
pub const PARTICIPANT_FIELDS: [(&str, &str); 6] = [
    ("readers", "Readers"),
    ("ushers", "Ushers"),
    ("greeters", "Greeters"),
    ("communion_assistants", "Communion assistants"),
    ("childrens_message", "Children's message"),
    ("acolytes", "Acolytes"),
];

/// The token a template prints one serving line's names
/// with: `{Sunday.<slug>}`.
pub fn participant_token(slug: &str) -> String {
    format!("{{Sunday.{slug}}}")
}

/// The slug a serving line's display name derives:
/// lowercase, every run of non-alphanumeric characters
/// becomes one underscore, no underscore at either end
/// ("Children's message" derives "children_s_message";
/// its permanent slug is "childrens_message", which is
/// why the built-in alias table exists).
pub fn serving_slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut pending_underscore = false;
    for c in name.chars() {
        if c.is_alphanumeric() {
            if pending_underscore && !out.is_empty() {
                out.push('_');
            }
            pending_underscore = false;
            out.extend(c.to_lowercase());
        } else {
            pending_underscore = true;
        }
    }
    out
}

/// The built-in slug for a built-in display name, matched
/// case-insensitively ("acolytes" for "Acolytes"), or
/// None when the name is the church's own line.
pub fn builtin_serving_slug(name: &str) -> Option<&'static str> {
    PARTICIPANT_FIELDS
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(name.trim()))
        .map(|(slug, _)| *slug)
}

/// The built-in display name for a built-in slug.
pub fn builtin_serving_name(slug: &str) -> Option<&'static str> {
    PARTICIPANT_FIELDS
        .iter()
        .find(|(s, _)| *s == slug)
        .map(|(_, name)| *name)
}

/// The canonical slug a serving slug resolves under:
/// a built-in line's permanent slug when `slug` is
/// that slug or the slug its display name derives,
/// else the slug itself. Alias records are keyed by
/// canonical slugs, so a token spelled either way
/// enters the same chain.
pub fn canonical_serving_slug(slug: &str) -> String {
    for (bslug, bname) in PARTICIPANT_FIELDS {
        if slug == bslug || slug == serving_slug(bname) {
            return bslug.to_string();
        }
    }
    slug.to_string()
}

/// The slug a display name's line is keyed under in
/// the alias record: its built-in canonical slug when
/// the name is a built-in line, else the slug the
/// name derives.
fn serving_line_key(name: &str) -> String {
    match builtin_serving_slug(name) {
        Some(bslug) => bslug.to_string(),
        None => serving_slug(name),
    }
}

/// Follow an alias chain from `start` to the slug it
/// currently ends at: each hop is looked up by the
/// slug itself and by its canonical form. A cycle
/// ends the walk at the first repeated slug, and the
/// walk stops after 64 hops, so a crafted profile
/// cannot loop it.
fn alias_chain_end(aliases: &BTreeMap<String, String>, start: &str) -> String {
    let mut visited: Vec<String> = vec![start.to_string()];
    let mut current = start.to_string();
    loop {
        let next = aliases
            .get(&current)
            .or_else(|| aliases.get(&canonical_serving_slug(&current)))
            .cloned();
        let Some(next) = next else {
            return current;
        };
        if visited.contains(&next) || visited.len() > 64 {
            return current;
        }
        visited.push(next.clone());
        current = next;
    }
}

/// The serving lines a church starts with: the six
/// built-in display names, in their built-in order.
pub fn default_serving_lines() -> Vec<String> {
    PARTICIPANT_FIELDS
        .iter()
        .map(|(_, name)| name.to_string())
        .collect()
}

/// The names only in one of the two serving-line
/// lists, trimmed, in list order: the first list's
/// names the second list does not carry.
fn serving_names_only_in<'a>(list: &'a [String], other: &[String]) -> Vec<&'a str> {
    list.iter()
        .map(|n| n.trim())
        .filter(|n| !n.is_empty() && !other.iter().any(|m| m.trim() == *n))
        .collect()
}

/// The storage keys one serving line's names can sit
/// under: the slug of the name as shown, plus its
/// built-in canonical slug when the name is a
/// built-in and the two differ.
fn serving_participant_keys(name: &str) -> Vec<String> {
    let mut keys = vec![serving_slug(name)];
    if let Some(bslug) = builtin_serving_slug(name) {
        if !keys.contains(&bslug.to_string()) {
            keys.push(bslug.to_string());
        }
    }
    keys
}

/// How a profile save's two serving-line lists pair
/// up (round 5, R5-C; segment pairing since round 6,
/// R6-C). A pairing is followed only when it is
/// UNAMBIGUOUS from the two lists alone:
///
/// - the names present in BOTH lists are anchors, in
///   order. The runs of old-only names and new-only
///   names between consecutive anchors (and at the
///   ends) are segments. Within a segment, equal
///   counts pair in order (the in-place case, now
///   immune to additions and removals in OTHER
///   segments); a segment with only new names is
///   additions; a segment with only old names is
///   removals;
/// - exactly one old-only name and one new-only name
///   in the whole edit pair with each other wherever
///   either sits (a single rename, with or without a
///   move — even across an anchor).
///
/// After segment pairing, if unpaired old-only AND
/// new-only names both remain — a rename crossed an
/// anchor, so its partner sits in another segment —
/// the pairing is AMBIGUOUS and nothing is followed
/// at all: guessing is how two lines' names crossed in
/// the round-5 F3 probe. The save is refused as a
/// whole (see [`migrate_serving_line_renames_detailed`]).
/// Pure reorder or swap has no old-only or new-only
/// names at all, so nothing pairs and nothing is
/// refused. Recorded residual (round 5, held): a
/// rename-only cycle inside one segment pairs in
/// order like in-place renames; the lists alone
/// cannot tell it apart from them, and its cure is
/// row identity in the editor phase.
///
/// An ambiguous pairing still RECORDS what it
/// computed before refusing — the segment pairs and
/// the unpaired names on both sides — because two
/// readers need the intent even though nothing may
/// be followed: the slug-conflict check excuses a
/// line renaming itself out of those names (round 7,
/// R7-9), and the migration reports the kinds of
/// change the pairing saw (round 7, R7-10).
struct ServingPairing {
    /// (old name, new name) rename pairs to follow.
    pairs: Vec<(String, String)>,
    /// Old-only names with no partner anywhere: the
    /// lines are removed.
    removals: Vec<String>,
    /// True when old-only and new-only names both
    /// remain unpaired: the change is refused whole.
    ambiguous: bool,
    /// Old-only names the segment pairing left
    /// unpaired, kept on an ambiguous pairing as the
    /// rename intent the pairing computed (round 7,
    /// R7-9). Empty on an unambiguous pairing, whose
    /// unpaired old names are folded into `removals`.
    unpaired_old: Vec<String>,
    /// New-only names the segment pairing left
    /// unpaired, the other half of that intent.
    unpaired_new: Vec<String>,
    /// On an ambiguous pairing, the kinds of change
    /// the pairing saw; None otherwise.
    ambiguous_kinds: Option<ServingLineEditKinds>,
    /// Every segment's runs of lines AS THEY STAND
    /// in the segment, in segment order — kept names
    /// included, as in-segment anchors (round 9,
    /// R9-15): the positional evidence the
    /// slug-conflict check judges from (round 8,
    /// R8-11) — within a segment, a new name sits at
    /// the position of the old name it is paired
    /// with, or, in a segment the pairing could not
    /// resolve, at the position of the line the
    /// pairing's own in-order reading assigns it.
    /// Positions are read on these unfiltered runs:
    /// on the filtered runs a kept name that moved
    /// into the segment shifted every later position
    /// and the evidence named the wrong line.
    segments: Vec<(Vec<String>, Vec<String>)>,
}

/// The kinds of change one serving-line edit
/// contains, as the pairing saw them (round 7,
/// R7-10): the refusal sentence for an ambiguous
/// edit names only the kinds that are present, so a
/// rename-plus-removal refusal no longer claims a
/// move that never happened. The kinds are computed
/// from the pairing's PER-SEGMENT outcomes (round
/// 9, R9-14), not from the two lists' global totals
/// plus a surplus inference: each segment
/// contributes the kinds its own evidence shows —
/// the renames it paired or left unresolved, the
/// removals and additions it resolved. A segment
/// the pairing resolved on its own — one holding
/// only removals between anchors, or only
/// additions — names its own kind and is never a
/// partner reservoir. `moved` is the pairing's own
/// crossing signal, claimed only when names
/// demonstrably crossed segments: unresolved names
/// spread over more than one segment, whose surplus
/// and deficit answer each other, or a surplus of
/// new names in an unresolved segment whose
/// partners can only sit in the segment after the
/// last anchor — the one old-only segment whose
/// names such a surplus can pair with, up to as
/// many names as the surplus needs (the canonical
/// crossed renames); that segment's names past the
/// need are removals. A surplus with no partner
/// segment to sit in is an addition, and a deficit
/// left over after any crossing is a removal. The
/// accounting is per occurrence (round 11,
/// R11-D): an occurrence whose name a pairing
/// already spent cannot also feed the reservoir —
/// reservoir absorption consumes unspent
/// occurrences only, so a duplicate name paired
/// in one segment and standing removal-only or
/// addition-only in another still shows its
/// removal or addition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServingLineEditKinds {
    /// At least one line was renamed.
    pub renamed: bool,
    /// At least one line was removed.
    pub removed: bool,
    /// At least one line was added.
    pub added: bool,
    /// At least one line's partner sits in another
    /// segment: the change crossed an anchor.
    pub moved: bool,
}

fn serving_pairing(old_lines: &[String], new_lines: &[String]) -> ServingPairing {
    let old_only = serving_names_only_in(old_lines, new_lines);
    let new_only = serving_names_only_in(new_lines, old_lines);
    if old_only.is_empty() && new_only.is_empty() {
        return ServingPairing {
            pairs: Vec::new(),
            removals: Vec::new(),
            ambiguous: false,
            unpaired_old: Vec::new(),
            unpaired_new: Vec::new(),
            ambiguous_kinds: None,
            segments: Vec::new(),
        };
    }
    if old_only.len() == 1 && new_only.len() == 1 {
        let pair = (old_only[0].to_string(), new_only[0].to_string());
        return ServingPairing {
            pairs: vec![pair],
            removals: Vec::new(),
            ambiguous: false,
            unpaired_old: Vec::new(),
            unpaired_new: Vec::new(),
            ambiguous_kinds: None,
            segments: Vec::new(),
        };
    }
    // Anchors: names present in both lists, matched
    // in order (a kept line that moved ahead of an
    // earlier anchor simply does not anchor; it is
    // in both lists, so it is in no segment's runs).
    let old_trim: Vec<String> = old_lines
        .iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    let new_trim: Vec<String> = new_lines
        .iter()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    let mut anchors: Vec<(usize, usize)> = Vec::new();
    let mut new_from = 0usize;
    for (oi, name) in old_trim.iter().enumerate() {
        if !new_trim.contains(name) {
            continue;
        }
        if let Some(rel) = new_trim[new_from..].iter().position(|n| n == name) {
            let ni = new_from + rel;
            anchors.push((oi, ni));
            new_from = ni + 1;
        }
    }
    // Segments between consecutive anchors, plus the
    // two ends.
    let mut segments: Vec<((usize, usize), (usize, usize))> = Vec::new();
    let mut o_start = 0usize;
    let mut n_start = 0usize;
    for &(oi, ni) in &anchors {
        segments.push(((o_start, oi), (n_start, ni)));
        o_start = oi + 1;
        n_start = ni + 1;
    }
    segments.push(((o_start, old_trim.len()), (n_start, new_trim.len())));
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut removals: Vec<String> = Vec::new();
    let mut leftover_old: Vec<String> = Vec::new();
    let mut leftover_new: Vec<String> = Vec::new();
    let mut segment_runs: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    // The segment tallies the ambiguous kinds are
    // read from (round 7, R7-10; round 8, R8-10;
    // round 9, R9-14; round 10, R10-8): each segment
    // contributes the kinds its own evidence shows,
    // tallied here — the surplus and deficit an
    // unresolved segment was left holding, the names
    // a removal-only segment resolved (kept apart
    // for the segment after the last anchor, the
    // trailing segment, though since round 10 a
    // surplus name's partner can sit in a removal-
    // only segment anywhere in the list, not only
    // there), and the names an addition-only segment
    // stood holding anywhere (a deficit old name's
    // possible partners, the mirror image).
    let mut leftover_segments = 0usize;
    let mut surplus_new_excess = 0usize;
    let mut deficit_old_excess = 0usize;
    // The removal-only and addition-only tallies
    // keep their NAMES (round 11, R11-D): the
    // reservoir below spends occurrences, and an
    // occurrence whose name a pairing already spent
    // cannot be spent again by the reservoir, so a
    // bare count is not enough to judge eligibility.
    let mut bounded_removal_names: Vec<String> = Vec::new();
    let mut trailing_removal_names: Vec<String> = Vec::new();
    let mut addition_only_names: Vec<String> = Vec::new();
    for (seg_index, ((o0, o1), (n0, n1))) in segments.iter().enumerate() {
        let seg_old: Vec<String> = old_trim[*o0..*o1]
            .iter()
            .filter(|n| !new_trim.contains(n))
            .cloned()
            .collect();
        let seg_new: Vec<String> = new_trim[*n0..*n1]
            .iter()
            .filter(|n| !old_trim.contains(n))
            .cloned()
            .collect();
        // The runs the pairing stores are the
        // segment's lines AS THEY STAND, kept names
        // included (round 9, R9-15): the slug-conflict
        // check reads positions from them, and a kept
        // name standing inside the segment is an
        // in-segment anchor for those positions. The
        // pairing itself works on the filtered runs
        // above, as it always has.
        segment_runs.push((old_trim[*o0..*o1].to_vec(), new_trim[*n0..*n1].to_vec()));
        if seg_old.len() == seg_new.len() {
            for (o, n) in seg_old.iter().zip(seg_new.iter()) {
                pairs.push((o.clone(), n.clone()));
            }
        } else if seg_new.is_empty() {
            if !seg_old.is_empty() {
                if seg_index + 1 == segments.len() {
                    trailing_removal_names.extend(seg_old.iter().cloned());
                } else {
                    bounded_removal_names.extend(seg_old.iter().cloned());
                }
            }
            removals.extend(seg_old);
        } else if seg_old.is_empty() {
            // Additions only: nothing to pair or clear.
            addition_only_names.extend(seg_new);
        } else {
            leftover_segments += 1;
            if seg_new.len() > seg_old.len() {
                surplus_new_excess += seg_new.len() - seg_old.len();
            } else {
                deficit_old_excess += seg_old.len() - seg_new.len();
            }
            leftover_old.extend(seg_old);
            leftover_new.extend(seg_new);
        }
    }
    if !leftover_old.is_empty() && !leftover_new.is_empty() {
        // Refused whole — but the computed pairs and
        // the unpaired names are kept on the pairing,
        // as the record of the intent the pairing saw
        // (round 7, R7-9, R7-10). Nothing follows
        // them: the migration checks `ambiguous` first.
        // The kinds are the segments' own outcomes
        // (round 9, R9-14), not the two lists' global
        // totals plus a surplus inference — both
        // round-9 lanes found shapes that inference
        // misdescribed in mirror directions. Surplus
        // new names and deficit old names in
        // unresolved segments first answer each
        // other: a surplus and a deficit spread over
        // segments are the same crossed renames, and
        // claim neither an addition nor a removal. A
        // surplus left over can still pair across an
        // anchor with a removal-only segment's old
        // names (the canonical crossed renames, whose
        // home was the trailing segment), up to as
        // many of those names as the surplus needs;
        // any further surplus is an addition. Since
        // round 10 (R10-8) the reservoir is counted
        // by name ANYWHERE in the list, not only in
        // the trailing segment, and it runs in both
        // directions: a deficit left over can pair
        // across an anchor with an addition-only
        // segment's new names the same way. The
        // round-9 account let a removal-only segment
        // sitting between anchors assert a plain
        // removal a standing surplus could have
        // absorbed, and read the two trailing mirror
        // geometries — surplus plus a trailing
        // removal-only segment, deficit plus a
        // trailing addition-only segment — by
        // opposite rules; the kinds now claim only
        // what the pairing establishes, and mirror
        // geometries read identically. Removal-only
        // names past every partner's need are
        // removals, as are addition-only names past
        // a deficit's need and a deficit left over
        // after the crossing and the reservoir.
        let crossed_pairs = surplus_new_excess.min(deficit_old_excess);
        let surplus_after_cross = surplus_new_excess - crossed_pairs;
        let deficit_after_cross = deficit_old_excess - crossed_pairs;
        // Per-occurrence accounting (round 11,
        // R11-D): the reservoir absorbs only
        // occurrences no pairing already spent. A
        // serving list can carry the same name in
        // two segments — one occurrence paired in
        // its segment, another standing removal-only
        // or addition-only in a different segment.
        // The paired occurrence and the reservoir
        // candidate share one name, and the name's
        // stored names and alias can be spent only
        // once, so an occurrence whose name a
        // pairing already spent (on the old side for
        // the removal reservoir, on the new side
        // for the addition reservoir) is not the
        // reservoir's to absorb: it stands as the
        // removal or addition it is. With unique
        // names no removal-only or addition-only
        // name is ever a paired name, so every
        // stated-rule shape reads exactly as before.
        let removal_only_total = bounded_removal_names.len() + trailing_removal_names.len();
        let unspent_removal_names = bounded_removal_names
            .iter()
            .chain(trailing_removal_names.iter())
            .filter(|name| !pairs.iter().any(|(old_name, _)| old_name == *name))
            .count();
        let unspent_addition_names = addition_only_names
            .iter()
            .filter(|name| !pairs.iter().any(|(_, new_name)| new_name == *name))
            .count();
        let reservoir_used = unspent_removal_names.min(surplus_after_cross);
        let surplus_final = surplus_after_cross - reservoir_used;
        let addition_reservoir_used = unspent_addition_names.min(deficit_after_cross);
        let deficit_final = deficit_after_cross - addition_reservoir_used;
        let kinds = ServingLineEditKinds {
            renamed: true,
            removed: removal_only_total > reservoir_used || deficit_final > 0,
            added: addition_only_names.len() > addition_reservoir_used || surplus_final > 0,
            moved: leftover_segments >= 2 || reservoir_used > 0 || addition_reservoir_used > 0,
        };
        return ServingPairing {
            pairs,
            removals,
            ambiguous: true,
            unpaired_old: leftover_old,
            unpaired_new: leftover_new,
            ambiguous_kinds: Some(kinds),
            segments: segment_runs,
        };
    }
    removals.extend(leftover_old);
    ServingPairing {
        pairs,
        removals,
        ambiguous: false,
        unpaired_old: Vec::new(),
        unpaired_new: Vec::new(),
        ambiguous_kinds: None,
        segments: segment_runs,
    }
}

/// One pairing or removal of an already-paired
/// name that a profile save's rename migration did
/// not follow (round 12, R12-11): a name's stored
/// names and alias can be spent only once, so the
/// first pairing in segment order is the pairing
/// actually made, and a later pairing of the same
/// old name — or a removal of it — is not followed.
/// The migration still reports the event, so the
/// app's status can name it and the office can
/// correct the names by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingLineUnfollowed {
    /// The old name that was paired more than once,
    /// or paired and also removed.
    pub old_name: String,
    /// The new name in the pairing that was kept:
    /// the pairing actually made for `old_name`.
    pub kept_new_name: String,
    /// The new name in the pairing that was not
    /// followed, or None when the event not followed
    /// was a removal of `old_name`.
    pub unfollowed_new_name: Option<String>,
}

/// What a profile save's rename migration did, in
/// the names the app must report (round 5, R5-C;
/// refusal since round 6, R6-C).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServingLineMigration {
    /// Old-only lines whose stored names were
    /// cleared because the line is gone. Only lines
    /// that actually held names are listed.
    pub removed: Vec<String>,
    /// Pairings and removals the migration did not
    /// follow because the old name was already spent
    /// by an earlier pairing (round 12, R12-11), in
    /// segment order — the order the office wrote
    /// them (round 13, R13-7b). A later pairing to
    /// the SAME target as the kept pairing is not
    /// recorded at all: its end state is the single
    /// pairing's (round 13, R13-7a). Empty when
    /// every pairing and removal was followed.
    pub unfollowed: Vec<ServingLineUnfollowed>,
    /// True when the pairing was ambiguous (renames
    /// crossed anchors): NOTHING was moved, cleared,
    /// or aliased, and the app refuses the
    /// serving-lines change as a whole, leaving the
    /// saved list exactly as it was.
    pub ambiguous: bool,
    /// When `ambiguous` is true, the kinds of change
    /// the pairing saw in the refused edit (round 7,
    /// R7-10), so the app's refusal sentence can name
    /// only what actually happened. None on a pairing
    /// that was followed.
    pub ambiguous_kinds: Option<ServingLineEditKinds>,
}

/// Move stored serving names across a profile edit
/// that renamed or removed lines (round 3, R3-3;
/// set-based since round 4, R4-2; unambiguous-only
/// since round 5, R5-C; segment pairing, recorded
/// aliases, and whole-change refusal since round 6,
/// R6-C). The pairing is `serving_pairing`'s: names
/// present in both lists anchor segments, and only
/// unambiguous pairs are followed. Each followed
/// pair's value, stored under the old name's keys
/// (its shown slug, and its built-in canonical slug
/// when the old name is a built-in), moves to the
/// new name's slug, AND the pairing is recorded in
/// `aliases` (the old line's key slug to the new
/// line's slug), so the old line's printed token
/// keeps resolving to the renamed line through
/// [`Document::participant_value`]. When the new
/// name's slug already holds a DIFFERENT value, both
/// values stay where they are: the target's value is
/// the fresher typing, and the source value keeps
/// its old key rather than being discarded. No
/// branch drops a value silently.
///
/// When the pairing is ambiguous, NOTHING is moved,
/// cleared, or aliased: the migration reports
/// `ambiguous` and the app refuses the serving-lines
/// change as a whole, so no name is left stranded
/// under a dead name (the round-5 "unfollowed"
/// disposition, and its status word, are gone).
///
/// An old-only name with no partner anywhere is a
/// REMOVED line (deleted outright, or renamed onto
/// a line that already exists): its stored values
/// are cleared, all its keys, any alias pointing at
/// its slug (or keyed by it) is dropped so its names
/// cannot reattach later, and its name is reported
/// as removed — in the same save as any successful
/// renames. Only names that actually held stored
/// values are reported. Pure over the two lists, the
/// map, and the alias record, so the rule is
/// testable without the app; the app calls the
/// detailed form when the profile's serving lines
/// are saved, and words the reported names in its
/// status line.
pub fn migrate_serving_line_renames_detailed(
    old_lines: &[String],
    new_lines: &[String],
    participants: &mut BTreeMap<String, String>,
    aliases: &mut BTreeMap<String, String>,
) -> ServingLineMigration {
    let pairing = serving_pairing(old_lines, new_lines);
    if pairing.ambiguous {
        return ServingLineMigration {
            removed: Vec::new(),
            unfollowed: Vec::new(),
            ambiguous: true,
            ambiguous_kinds: pairing.ambiguous_kinds,
        };
    }
    // Per-occurrence accounting (round 11,
    // R11-D): a name standing in two segments can
    // be paired twice, or paired in one segment
    // and removal-only in another, but its stored
    // names sit under one slug and its alias under
    // one key, so the name can be spent only once.
    // The first pairing in segment order is the
    // pairing actually made: it moves the stored
    // names and records the alias. A later pairing
    // of the same old name is not followed — it
    // would move nothing (the source is spent) yet
    // overwrite the recorded alias, leaving the
    // participant map and the alias record
    // disagreeing about where the name went — and
    // a removal of the same name is not followed
    // either: it would clear the alias and the
    // names the pairing just made. The recorded
    // aliases correspond one-to-one with the
    // pairings actually made. Since round 12
    // (R12-11) the events not followed are carried
    // on the migration as `unfollowed`, so the save
    // speaks about them; the accounting itself is
    // unchanged.
    // The pairing each spent name was spent on, so
    // an unfollowed event can name the pairing that
    // was kept (round 12, R12-11).
    let mut spent_old_names: Vec<(&str, &str)> = Vec::new();
    let mut migration = ServingLineMigration::default();
    for (old_name, new_name) in &pairing.pairs {
        if let Some((_, kept_new_name)) = spent_old_names
            .iter()
            .find(|(spent_name, _)| *spent_name == old_name.as_str())
        {
            // Round 13 (R13-7a): a later pairing to
            // the SAME target as the kept pairing is
            // not an event at all — the end state is
            // the single pairing's, and naming it as
            // not followed would be a false alarm
            // inviting a by-hand correction of names
            // that are already right. The accounting
            // is unchanged: the pairing is still not
            // followed (the name is spent); it is
            // simply not recorded.
            if new_name == kept_new_name {
                continue;
            }
            // Not followed — but not silent (round
            // 12, R12-11): the detail carries the
            // event, naming the name, the pairing
            // that was kept, and this pairing.
            let noted_old_name = old_name.clone();
            let noted_kept_new_name = kept_new_name.to_string();
            let noted_unfollowed_new_name = Some(new_name.clone());
            let event = ServingLineUnfollowed {
                old_name: noted_old_name,
                kept_new_name: noted_kept_new_name,
                unfollowed_new_name: noted_unfollowed_new_name,
            };
            if !migration.unfollowed.contains(&event) {
                migration.unfollowed.push(event);
            }
            continue;
        }
        spent_old_names.push((old_name.as_str(), new_name.as_str()));
        move_participant_value(old_name, new_name, participants);
        let old_key = serving_line_key(old_name);
        let new_slug = serving_slug(new_name);
        if old_key != new_slug {
            aliases.insert(old_key, new_slug);
        }
    }
    for old_name in &pairing.removals {
        if let Some((_, kept_new_name)) = spent_old_names
            .iter()
            .find(|(spent_name, _)| *spent_name == old_name.as_str())
        {
            // The removal is not followed either, and
            // is reported the same way (round 12,
            // R12-11).
            let noted_old_name = old_name.clone();
            let noted_kept_new_name = kept_new_name.to_string();
            let event = ServingLineUnfollowed {
                old_name: noted_old_name,
                kept_new_name: noted_kept_new_name,
                unfollowed_new_name: None,
            };
            if !migration.unfollowed.contains(&event) {
                migration.unfollowed.push(event);
            }
            continue;
        }
        let keys = serving_participant_keys(old_name);
        let mut held = false;
        for k in &keys {
            if participants.remove(k).is_some() {
                held = true;
            }
        }
        // The removed line's slugs, in every spelling
        // an alias could use: drop any alias pointing
        // at the line, and the line's own outgoing
        // alias, so a later line reusing the name
        // starts fresh and inherits nothing.
        let mut slugs = keys.clone();
        let key = serving_line_key(old_name);
        if !slugs.contains(&key) {
            slugs.push(key);
        }
        aliases.retain(|from, to| !slugs.contains(to) && !slugs.contains(from));
        if held {
            migration.removed.push(old_name.to_string());
        }
    }
    // Round 13 (R13-7b): the unfollowed events are
    // carried in SEGMENT ORDER — the order the
    // office wrote them — not pairings-then-
    // removals. The accounting above is unchanged:
    // pairings were still followed (and spent)
    // before removals were judged, exactly as
    // before; only the order the events are
    // carried in changes, so the app's status
    // words them as they were written. Each
    // event's place is the place of its own item
    // in the pairing's segment runs, recomputed
    // here on the filtered runs the pairing
    // itself paired from; an event no segment
    // item matches keeps its collected place,
    // after every matched event.
    if migration.unfollowed.len() > 1 {
        let old_all: Vec<String> = old_lines
            .iter()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .collect();
        let new_all: Vec<String> = new_lines
            .iter()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .collect();
        // The pairing's items in segment order:
        // (the new name, the old name) for a
        // pairing, (None, the old name) for a
        // removal.
        let mut order_items: Vec<(Option<String>, String)> = Vec::new();
        for (seg_old_run, seg_new_run) in &pairing.segments {
            let seg_old: Vec<&String> = seg_old_run
                .iter()
                .filter(|n| !new_all.contains(n))
                .collect();
            let seg_new: Vec<&String> = seg_new_run
                .iter()
                .filter(|n| !old_all.contains(n))
                .collect();
            if seg_old.len() == seg_new.len() {
                for (o, n) in seg_old.iter().zip(seg_new.iter()) {
                    let item_new = Some((*n).clone());
                    let item_old = (*o).clone();
                    order_items.push((item_new, item_old));
                }
            } else if seg_new.is_empty() {
                for o in seg_old {
                    let item_old = o.clone();
                    order_items.push((None, item_old));
                }
            }
        }
        let mut keys: Vec<usize> = Vec::new();
        for event in &migration.unfollowed {
            let item = (event.unfollowed_new_name.clone(), event.old_name.clone());
            let key = order_items
                .iter()
                .position(|it| *it == item)
                .unwrap_or(usize::MAX);
            keys.push(key);
        }
        let mut order: Vec<usize> = (0..migration.unfollowed.len()).collect();
        order.sort_by_key(|&k| keys[k]);
        let collected = std::mem::take(&mut migration.unfollowed);
        let mut reordered: Vec<ServingLineUnfollowed> = Vec::new();
        for &k in &order {
            reordered.push(collected[k].clone());
        }
        migration.unfollowed = reordered;
    }
    migration
}

/// The removed-line half of
/// [`migrate_serving_line_renames_detailed`], kept
/// for the callers and tests that only ask what was
/// cleared.
pub fn migrate_serving_line_renames(
    old_lines: &[String],
    new_lines: &[String],
    participants: &mut BTreeMap<String, String>,
    aliases: &mut BTreeMap<String, String>,
) -> Vec<String> {
    migrate_serving_line_renames_detailed(old_lines, new_lines, participants, aliases).removed
}

/// One serving-line slug conflict in a profile
/// save's new list (round 5, R5-C): the two lines
/// the app's refusal sentence names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingLineSlugConflict {
    /// For two new-list lines deriving one slug: the
    /// first of them. For a built-in conflict: the
    /// taking line, identified distinguishably
    /// (round 8, R8-11) — the name it had in the old
    /// list, or, when the taking line is a new line
    /// or a line standing under the contested name
    /// itself, the contested name as typed.
    pub first: String,
    /// The other line: the twin in the new list, or,
    /// for a built-in conflict, the holding side —
    /// the name the bulletin currently prints for
    /// the claimed line (the claimant's current
    /// display name), never the built-in's own name
    /// unless the claimant currently stands under it.
    pub second: String,
    /// The slug both sides resolve through.
    pub slug: String,
    /// True when `second` is a built-in line's
    /// claimed place (the alias case); false for two
    /// new-list lines deriving the same slug.
    pub builtin: bool,
    /// True when the taking line is a new line in
    /// the new list (built-in conflicts only), so the
    /// refusal sentence can say "a new line" instead
    /// of naming a line that has no old name.
    pub taker_is_new: bool,
}

/// The slug conflict a profile save must refuse, if
/// any (round 5, R5-C, RT5-4; claimed-ness by
/// resolution since round 6, R6-C, R6-4).
/// `serving_slug` is many-to-one, so distinct
/// display names can share one storage slot and
/// silently share one set of names. The save
/// refuses when:
///
/// - two names in the new list derive the same
///   slug ("Prayer Team" and "Prayer-Team"), or
/// - a name in the new list derives the canonical
///   slug (or the shown-name slug) of a built-in
///   serving line while that line is not the line
///   that CLAIMS the slug, so the name would hijack
///   the built-in line's This Sunday row and its
///   printed `{Sunday.<slug>}` token.
///
/// Claimed-ness follows resolution, not spelling:
/// a built-in's canonical slug is claimed by the
/// OLD-list line that the built-in's token currently
/// resolves to — the line whose own key IS the
/// canonical slug (the built-in standing under its
/// own name, in either spelling), or the line whose
/// slug the profile's alias record maps the
/// canonical slug to, through its chain. Only the
/// claiming line may take a name deriving the
/// canonical slug, and the claiming line itself may
/// always rename to such a name — including back to
/// the built-in's own name. When no old-list line
/// claims the slug (the built-in line is gone and
/// no alias record survives), the name is free.
/// The conflict the check returns names the taking
/// line by the pairing's best evidence — the name it
/// had, or a new line — and the holding side by the
/// name the bulletin currently prints for the claimed
/// line, so a sentence built from it never names one
/// unqualified name on both sides (round 8, R8-11).
/// Pure over the two lists and the alias record;
/// the app calls it at the profile save and refuses
/// before anything changes.
pub fn serving_line_slug_conflict(
    old_lines: &[String],
    new_lines: &[String],
    aliases: &BTreeMap<String, String>,
) -> Option<ServingLineSlugConflict> {
    let names = serving_names_only_in(new_lines, &[]);
    // Two names the editor kept as distinct lines
    // deriving one slug share one storage slot.
    for (i, first) in names.iter().enumerate() {
        let first_slug = serving_slug(first);
        for second in names.iter().skip(i + 1) {
            if serving_slug(second) == first_slug {
                let conflict = ServingLineSlugConflict {
                    first: first.to_string(),
                    second: second.to_string(),
                    slug: first_slug,
                    builtin: false,
                    taker_is_new: false,
                };
                return Some(conflict);
            }
        }
    }
    let pairing = serving_pairing(old_lines, new_lines);
    for name in &names {
        let slug = serving_slug(name);
        if slug.is_empty() {
            continue;
        }
        for (bslug, bname) in PARTICIPANT_FIELDS {
            let derived = serving_slug(bname);
            if slug != bslug && slug != derived {
                continue;
            }
            let Some(claimant) = claiming_line(old_lines, aliases, bslug) else {
                // No old-list line holds this built-in's
                // token: the name takes no one's place.
                continue;
            };
            // The line itself: the claimant standing
            // under its current name, unchanged...
            if name.trim() == claimant {
                continue;
            }
            // Who takes this name, on the pairing's
            // best evidence (round 8, R8-11)? The
            // round-7 excuse tested set membership —
            // the claimant among the unpaired old
            // names, the contested name among the
            // unpaired new ones — which excused a
            // genuine hijack sharing those sets and
            // missed a claimant the pairing guessed
            // across inside an equal-count segment.
            // The judgment now works from who the
            // pairing assigns the name to: a followed
            // pair, or the line standing at the
            // name's own position inside its segment.
            // The excuse is reserved for the shapes
            // where that evidence assigns the name to
            // the claimant itself, or where no
            // positional evidence assigns it to a
            // different line. The excuse changes only
            // WHICH refusal the save meets — the
            // migration still refuses an ambiguous
            // edit whole — so the claimant is never
            // reported as conflicting with itself.
            let contested = name.trim();
            let mut taker: Option<(String, bool)> = None;
            let mut excused = false;
            if pairing
                .pairs
                .iter()
                .any(|(o, n)| o == claimant && n == contested)
            {
                // The claimant renaming to this name
                // in a pairing that would be followed.
                excused = true;
            } else if let Some((o, _)) = pairing.pairs.iter().find(|(_, n)| n == contested) {
                // A followed pair assigns the name to
                // a different line: that line takes it.
                taker = Some((o.clone(), false));
            } else {
                // Positional evidence inside the
                // segments: the contested name at
                // position i of a segment's new run
                // sits on the line at position i of
                // its old run, resolved segment or
                // not; past the old run's end, it
                // arrives on a new line.
                for (seg_old, seg_new) in &pairing.segments {
                    let Some(idx) = seg_new.iter().position(|n| n == contested) else {
                        continue;
                    };
                    if idx < seg_old.len() {
                        if seg_old[idx] == claimant {
                            excused = true;
                        } else {
                            taker = Some((seg_old[idx].clone(), false));
                        }
                    } else {
                        taker = Some((contested.to_string(), true));
                    }
                    break;
                }
            }
            if !excused && taker.is_none() {
                if old_lines.iter().any(|l| l.trim() == contested) {
                    // A line standing under this very
                    // name in the old list, unchanged:
                    // it is the taker, under its own
                    // name.
                    taker = Some((contested.to_string(), false));
                } else if pairing.ambiguous
                    && pairing.unpaired_old.iter().any(|o| o == claimant)
                    && pairing.unpaired_new.iter().any(|n| n == contested)
                {
                    // The membership excuse, kept for
                    // the shapes it was pinned on
                    // (round 7, R7-9) and only where no
                    // positional evidence above assigned
                    // the name to a different line.
                    excused = true;
                } else {
                    // No evidence assigns the name to
                    // any old line: it arrives on a
                    // new line.
                    taker = Some((contested.to_string(), true));
                }
            }
            if excused {
                continue;
            }
            let (taker_label, taker_is_new) =
                taker.unwrap_or_else(|| (contested.to_string(), false));
            let holder = claimant.to_string();
            let conflict = ServingLineSlugConflict {
                first: taker_label,
                second: holder,
                slug,
                builtin: true,
                taker_is_new,
            };
            return Some(conflict);
        }
    }
    None
}

/// The OLD-list line that currently claims a
/// built-in's canonical slug (round 6, R6-C): the
/// line whose own key is the canonical slug, or the
/// line whose slug the alias chain from the
/// canonical slug ends at. None when no old-list
/// line holds the built-in's token.
fn claiming_line<'a>(
    old_lines: &'a [String],
    aliases: &BTreeMap<String, String>,
    bslug: &str,
) -> Option<&'a str> {
    for line in old_lines {
        let shown = line.trim();
        if !shown.is_empty() && serving_line_key(shown) == bslug {
            return Some(shown);
        }
    }
    let end = alias_chain_end(aliases, bslug);
    if end != bslug {
        for line in old_lines {
            let shown = line.trim();
            if !shown.is_empty() && (serving_slug(shown) == end || serving_line_key(shown) == end) {
                return Some(shown);
            }
        }
    }
    None
}

/// Move one renamed line's stored value to its new
/// name's slug, under the keeping rules
/// `migrate_serving_line_renames` states: the value
/// comes from the first of the old name's keys that
/// holds one (shown slug first, canonical slug
/// second); a target already holding a different
/// value keeps it, and the source value then stays
/// under its old key; source keys holding the moved
/// value are cleared, while a source key holding a
/// different value keeps it, so no stored name is
/// ever discarded by a rename.
fn move_participant_value(
    old_name: &str,
    new_name: &str,
    participants: &mut BTreeMap<String, String>,
) {
    let target = serving_slug(new_name);
    let mut sources: Vec<String> = vec![serving_slug(old_name)];
    if let Some(bslug) = builtin_serving_slug(old_name) {
        if !sources.contains(&bslug.to_string()) {
            sources.push(bslug.to_string());
        }
    }
    let found = sources
        .iter()
        .filter(|k| k.as_str() != target)
        .find_map(|k| participants.get(k).cloned());
    let Some(value) = found else {
        return;
    };
    if let Some(existing) = participants.get(&target) {
        if *existing != value {
            // Both values stay where they are.
            return;
        }
    }
    participants.insert(target.clone(), value.clone());
    for k in &sources {
        if *k != target && participants.get(k).map(String::as_str) == Some(value.as_str()) {
            participants.remove(k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolve_fields, ChurchProfile, Document, SheetSpec, Size, FORMAT_VERSION};
    use std::collections::BTreeMap;

    fn bare_doc() -> Document {
        Document {
            format_version: FORMAT_VERSION,
            title: "t".into(),
            profile: ChurchProfile::default(),
            fields: BTreeMap::new(),
            styles: vec![],
            stories: vec![],
            panels: vec![],
            sheet: SheetSpec {
                preset: "letter-portrait".into(),
                size: Size::new(0, 0),
            },
            no_announcements: false,
            assets: Vec::new(),
            asset_bytes: Default::default(),
            occasion_time: None,
            wanted_fonts: Vec::new(),
            participants: BTreeMap::new(),
            inserts: Vec::new(),
            applied_contributions: Vec::new(),
        }
    }

    #[test]
    fn participant_tokens_resolve_with_and_without_values() {
        let mut doc = bare_doc();
        // Unset: every participant token resolves to nothing,
        // exactly like the other weekly fields.
        assert_eq!(
            resolve_fields(
                "Readers: {Sunday.readers}|Ushers: {Sunday.ushers}|Greeters: {Sunday.greeters}|Communion: {Sunday.communion_assistants}",
                &doc
            ),
            "Readers: |Ushers: |Greeters: |Communion: "
        );
        doc.fields
            .insert("readers".to_string(), "Ana Ruiz".to_string());
        doc.fields
            .insert("ushers".to_string(), "Tom and Bea".to_string());
        doc.fields.insert("greeters".to_string(), "Sam".to_string());
        doc.fields
            .insert("communion_assistants".to_string(), "Pat, Jo".to_string());
        assert_eq!(
            resolve_fields(
                "Readers: {Sunday.readers}|Ushers: {Sunday.ushers}|Greeters: {Sunday.greeters}|Communion: {Sunday.communion_assistants}",
                &doc
            ),
            "Readers: Ana Ruiz|Ushers: Tom and Bea|Greeters: Sam|Communion: Pat, Jo"
        );
        // The spec table's tokens and the resolver's agree.
        for (key, _label) in PARTICIPANT_FIELDS.iter().take(4) {
            let key = *key;
            let token = participant_token(key);
            assert!(doc.fields.contains_key(key), "test setup covers {key}");
            assert_eq!(
                resolve_fields(&token, &doc),
                doc.fields[key],
                "{token} resolves from {key}"
            );
        }
        // The two lines R-2b added resolve from the map.
        doc.set_participant_value("Children's message", "Ms. Dee");
        doc.set_participant_value("Acolytes", "Sam, Jo");
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Dee"
        );
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Sam, Jo");
    }

    #[test]
    fn legacy_participant_fields_fold_into_the_map_on_load() {
        // A file written before the participants map: the
        // four names live in `fields` under their slugs.
        let mut doc = bare_doc();
        doc.fields
            .insert("readers".to_string(), "Ana Ruiz".to_string());
        doc.fields
            .insert("ushers".to_string(), "Tom and Bea".to_string());
        doc.migrate_participants();
        assert!(!doc.fields.contains_key("readers"));
        assert_eq!(doc.participant_value("readers"), "Ana Ruiz");
        assert_eq!(
            resolve_fields("Readers: {Sunday.readers}", &doc),
            "Readers: Ana Ruiz"
        );
        // The fold is a fold, not a copy: running it again
        // changes nothing, and a document already speaking
        // in the map keeps its fields untouched.
        doc.fields
            .insert("greeters".to_string(), "Late".to_string());
        doc.migrate_participants();
        assert_eq!(doc.fields.get("greeters").map(String::as_str), Some("Late"));
        // ...and the leftover still resolves, through the
        // fallback, so no honest storage goes unread.
        assert_eq!(resolve_fields("{Sunday.greeters}", &doc), "Late");
    }

    #[test]
    fn a_renamed_serving_line_keeps_its_token() {
        // R-2b: the church renames "Acolytes" to "Candle
        // lighters". Names stored while the line showed
        // its built-in name sit under the built-in slug,
        // and the template's token must keep resolving.
        let mut doc = bare_doc();
        doc.set_participant_value("Acolytes", "Sam, Jo");
        doc.profile.serving_lines = vec!["Candle lighters".to_string()];
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Sam, Jo");
        // Names stored under the new name as shown resolve
        // under the new name's own token too.
        doc.set_participant_value("Candle lighters", "Rae");
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Rae");
        assert_eq!(resolve_fields("{Sunday.candle_lighters}", &doc), "Rae");
    }

    #[test]
    fn a_profile_rename_moves_the_stored_names() {
        // Round 3, R3-3: renaming "Acolytes" to "Candle
        // lighters" in the profile moves the stored names
        // to the renamed line: the row shows them, typing
        // replaces them, and the built-in token prints
        // the NEW names afterwards (the audit's gap was a
        // regression test that asserted only the row).
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Acolytes", "Sam, Jo");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[5] = "Candle lighters".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = new_lines;
        // The value moved; the built-in key is cleared.
        assert_eq!(
            doc.participants.get("candle_lighters").map(String::as_str),
            Some("Sam, Jo")
        );
        assert!(!doc.participants.contains_key("acolytes"));
        // The renamed row shows the names.
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Sam, Jo");
        // Typing new names in the renamed row replaces
        // them...
        doc.set_participant_value("Candle lighters", "Rae, Kim");
        assert_eq!(
            doc.participant_value_for_name("Candle lighters"),
            "Rae, Kim"
        );
        // ...and the template's built-in token prints the
        // NEW names, not the stranded ones.
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Rae, Kim");
        assert_eq!(resolve_fields("{Sunday.candle_lighters}", &doc), "Rae, Kim");
    }

    #[test]
    fn a_rename_moves_names_from_either_stored_key() {
        // "Children's message" is stored under its shown
        // slug (children_s_message) by the row, but a
        // value can also sit under the permanent slug
        // (childrens_message); a rename moves either.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.participants
            .insert("childrens_message".to_string(), "Ms. Dee".to_string());
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[4] = "Kids' message".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Kids' message"), "Ms. Dee");
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Dee"
        );
        assert!(!doc.participants.contains_key("childrens_message"));
        // A save with no renames moves nothing.
        doc.set_participant_value("Readers", "Ana Ruiz");
        let before = doc.participants.clone();
        let lines_now = doc.profile.serving_lines.clone();
        migrate_serving_line_renames(
            &lines_now,
            &lines_now,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(doc.participants, before);
    }

    #[test]
    fn a_swap_or_reorder_moves_and_crosses_nothing() {
        // Round 4, R4-2: the migration compares the
        // lists as sets of names. A name in both lists
        // is the same line wherever it sits, so tidying
        // the list's order loses nothing and crosses
        // nothing (the positional rule lost one line's
        // names and printed another's against the
        // wrong duty).
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let before = doc.participants.clone();
        let old_lines = doc.profile.serving_lines.clone();
        let mut swapped = old_lines.clone();
        swapped.swap(0, 5);
        let removed = migrate_serving_line_renames(
            &old_lines,
            &swapped,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(doc.participants, before);
        doc.profile.serving_lines = swapped;
        assert_eq!(doc.participant_value_for_name("Readers"), "Ann");
        assert_eq!(doc.participant_value_for_name("Acolytes"), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Bob");

        // A three-line reorder, the review lane's
        // traced shape, is the same story.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Rae");
        doc.set_participant_value("Ushers", "Uma");
        doc.set_participant_value("Greeters", "Gus");
        let before = doc.participants.clone();
        let old_lines = doc.profile.serving_lines.clone();
        let mut reordered = old_lines.clone();
        reordered.swap(1, 2);
        let removed = migrate_serving_line_renames(
            &old_lines,
            &reordered,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(doc.participants, before);
        doc.profile.serving_lines = reordered;
        assert_eq!(resolve_fields("{Sunday.ushers}", &doc), "Uma");
        assert_eq!(resolve_fields("{Sunday.greeters}", &doc), "Gus");
    }

    #[test]
    fn a_rename_with_a_reorder_lands_on_the_renamed_line() {
        // Round 4, R4-2: one edit that renames
        // "Acolytes" to "Candle lighters" AND moves
        // the line to the top of the list. The acolyte's
        // names land on the renamed line; the readers'
        // names stay on Readers, which also moved.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines.remove(5);
        new_lines.insert(0, "Candle lighters".to_string());
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(
            doc.participants.get("candle_lighters").map(String::as_str),
            Some("Bob")
        );
        assert_eq!(
            doc.participants.get("readers").map(String::as_str),
            Some("Ann")
        );
        assert!(!doc.participants.contains_key("acolytes"));
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Bob");
        assert_eq!(resolve_fields("{Sunday.candle_lighters}", &doc), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
    }

    #[test]
    fn a_rename_onto_a_filled_slug_keeps_both_values() {
        // Round 4, R4-2: when the renamed line's slug
        // already holds a different value (names typed
        // for that slug in an earlier week), both values
        // stay where they are: the target's value is
        // the fresher typing, and the renamed line's
        // value keeps its old key instead of being
        // discarded.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Acolytes", "Bob");
        doc.participants
            .insert("candle_lighters".to_string(), "Zed".to_string());
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[5] = "Candle lighters".to_string();
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
        assert_eq!(
            doc.participants.get("candle_lighters").map(String::as_str),
            Some("Zed")
        );
        assert_eq!(
            doc.participants.get("acolytes").map(String::as_str),
            Some("Bob")
        );
    }

    #[test]
    fn two_renames_with_a_reorder_cross_nothing_and_are_named() {
        // Round 5, F3, disposition changed in round 6
        // (R6-C, R6-5): Acolytes ("Bob") and Readers
        // ("Ann") are both renamed in one save, and
        // the renamed lines move across the list:
        // "Candle lighters" (the acolytes' new name)
        // lands at the top and "Lectors" (the readers'
        // new name) just under it, ahead of lines that
        // did not change. The renames cross anchors —
        // each partner sits in another segment — so
        // the pairing is ambiguous. Round 5 kept the
        // new names and reported the lines "unfollowed"
        // with their names stranded under the old
        // slugs; round 6 refuses the change as a
        // whole instead: nothing is paired, moved,
        // cleared, or aliased, the saved list keeps
        // the old names (the app does not apply the
        // edit), and the no-crossing, no-loss
        // properties this test was written for are
        // re-pinned under that disposition.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let before = doc.participants.clone();
        let old_lines = doc.profile.serving_lines.clone();
        let new_lines: Vec<String> = [
            "Candle lighters",
            "Lectors",
            "Ushers",
            "Greeters",
            "Communion assistants",
            "Children's message",
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(migration.ambiguous);
        assert!(migration.removed.is_empty());
        // Byte-identical afterwards: the map and the
        // alias record are untouched, and the saved
        // list still shows the old names, so the
        // names stay reachable by their lines and
        // their built-in tokens.
        assert_eq!(doc.participants, before);
        assert!(doc.profile.serving_aliases.is_empty());
        assert_eq!(doc.profile.serving_lines, old_lines);
        assert_eq!(doc.participant_value_for_name("Readers"), "Ann");
        assert_eq!(doc.participant_value_for_name("Acolytes"), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Bob");
        // The wrapper agrees: nothing was cleared,
        // and the map it was handed is untouched.
        let mut map = before.clone();
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let removed = migrate_serving_line_renames(&old_lines, &new_lines, &mut map, &mut aliases);
        assert!(removed.is_empty());
        assert_eq!(map, before);
        assert!(aliases.is_empty());
    }

    #[test]
    fn two_renames_in_place_pair_by_position() {
        // Round 5, R5-C: two renames in one save where
        // nothing moved (same length, every position
        // unchanged or the pair at that position) pair
        // by position and land on the right lines.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Bob");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[0] = "Lectors".to_string();
        new_lines[5] = "Candle lighters".to_string();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(migration.removed.is_empty());
        assert!(!migration.ambiguous);
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Lectors"), "Ann");
        assert_eq!(doc.participant_value_for_name("Candle lighters"), "Bob");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "Bob");
        // The token bridge stays honest: each renamed
        // line's row and its built-in token agree.
        assert_eq!(
            doc.participant_value_for_name("Candle lighters"),
            resolve_fields("{Sunday.acolytes}", &doc)
        );
    }

    #[test]
    fn a_rename_then_a_front_insertion_never_crosses_tokens() {
        // Round 6, RT6-5: rename Readers to Lectors
        // (one save), then insert a custom line "Band"
        // at the FRONT of the list (a second save, a
        // pure addition). The old position bridge then
        // paired {Sunday.readers} with the line now at
        // the readers' position and printed the band's
        // names for it, and the Lectors' names under
        // Ushers. With the bridge deleted, the recorded
        // alias carries the token: readers resolves to
        // the renamed line, ushers to the Ushers, and a
        // token for a line that does not exist resolves
        // empty, never to another line's names.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Laura Lector");
        doc.set_participant_value("Ushers", "Uma Usher");
        // Save one: Readers renamed to Lectors, in
        // place. The pairing is recorded as an alias.
        let old_lines = doc.profile.serving_lines.clone();
        let mut renamed = old_lines.clone();
        renamed[0] = "Lectors".to_string();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &renamed,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        doc.profile.serving_lines = renamed;
        assert_eq!(
            doc.profile
                .serving_aliases
                .get("readers")
                .map(String::as_str),
            Some("lectors")
        );
        // Save two: "Band" inserted at the front.
        let old_lines = doc.profile.serving_lines.clone();
        let mut with_band = old_lines.clone();
        with_band.insert(0, "Band".to_string());
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &with_band,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        doc.profile.serving_lines = with_band;
        doc.set_participant_value("Band", "The Band Kids");
        assert_eq!(doc.participant_value("readers"), "Laura Lector");
        assert_eq!(doc.participant_value("ushers"), "Uma Usher");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Laura Lector");
        assert_eq!(resolve_fields("{Sunday.ushers}", &doc), "Uma Usher");
        assert_eq!(resolve_fields("{Sunday.band}", &doc), "The Band Kids");
        assert_eq!(doc.participant_value("choir"), "");
    }

    #[test]
    fn an_alias_chain_across_two_renames_resolves() {
        // Round 6, R6-C: Readers to Lectors in one
        // save, Lectors to "Readers' leaders" in the
        // next. The alias record holds the chain
        // (readers to lectors, lectors to
        // readers_leaders), and the built-in token
        // follows it to the names.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        let old_lines = doc.profile.serving_lines.clone();
        let mut step_one = old_lines.clone();
        step_one[0] = "Lectors".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &step_one,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = step_one;
        let old_lines = doc.profile.serving_lines.clone();
        let mut step_two = old_lines.clone();
        step_two[0] = "Readers' leaders".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &step_two,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = step_two;
        assert_eq!(
            doc.profile
                .serving_aliases
                .get("readers")
                .map(String::as_str),
            Some("lectors")
        );
        assert_eq!(
            doc.profile
                .serving_aliases
                .get("lectors")
                .map(String::as_str),
            Some("readers_leaders")
        );
        assert_eq!(doc.participant_value("readers"), "Ann");
        assert_eq!(resolve_fields("{Sunday.readers}", &doc), "Ann");
        assert_eq!(doc.participant_value_for_name("Readers' leaders"), "Ann");
    }

    #[test]
    fn a_crafted_alias_cycle_resolves_empty_and_cannot_hang() {
        // Round 6, R6-C: a hand-edited profile whose
        // alias record loops (alpha to beta, beta to
        // alpha) must not hang resolution and must not
        // cross into another line's names. The pinned
        // rule: the walk stops at the first repeated
        // slug and the cycle resolves EMPTY.
        let mut doc = bare_doc();
        doc.profile
            .serving_aliases
            .insert("alpha".to_string(), "beta".to_string());
        doc.profile
            .serving_aliases
            .insert("beta".to_string(), "alpha".to_string());
        doc.set_participant_value("Greeters", "Gus");
        assert_eq!(doc.participant_value("alpha"), "");
        assert_eq!(doc.participant_value("beta"), "");
        // The honest line beside the cycle is unmoved.
        assert_eq!(doc.participant_value("greeters"), "Gus");
    }

    #[test]
    fn claimedness_follows_the_alias_record_both_ways() {
        // Round 6, R6-4: the built-in "Children's
        // message" was renamed to "Kids message" in an
        // earlier save, so the alias record claims its
        // slug for the Kids message line.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Children's message", "Ms Dee");
        let old_lines = doc.profile.serving_lines.clone();
        let mut renamed = old_lines.clone();
        renamed[4] = "Kids message".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &renamed,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = renamed;
        let lines_now = doc.profile.serving_lines.clone();
        // Another line taking the built-in's own name
        // is refused while Kids message holds the
        // claim: the token would print that line's
        // names as the children's message.
        let mut hijack = lines_now.clone();
        hijack[1] = "Children's message".to_string();
        let conflict =
            serving_line_slug_conflict(&lines_now, &hijack, &doc.profile.serving_aliases)
                .expect("ushers taking the claimed name conflicts");
        assert!(conflict.builtin);
        // FLAGGED, round 8 (R8-11): `first` used to be
        // the contested name itself and `second` the
        // built-in's display name, so this assertion
        // pair read "Children's message" on both
        // sides. The conflict now names the taking
        // line by the name it had and the holding
        // side by the name the bulletin currently
        // prints for the claimed line.
        assert_eq!(conflict.first, "Ushers");
        assert_eq!(conflict.second, "Kids message");
        // The claiming line itself may take the name
        // back, in either spelling.
        let mut back = lines_now.clone();
        back[4] = "Children's message".to_string();
        assert_eq!(
            serving_line_slug_conflict(&lines_now, &back, &doc.profile.serving_aliases),
            None
        );
        let mut back_plain = lines_now.clone();
        back_plain[4] = "Childrens Message".to_string();
        assert_eq!(
            serving_line_slug_conflict(&lines_now, &back_plain, &doc.profile.serving_aliases),
            None
        );
    }

    #[test]
    fn two_in_place_renames_and_an_added_line_all_take_effect() {
        // Round 6, R6-5: two lines renamed in place
        // and one line added, all in one save. The
        // round-5 pairing compared raw lists, read
        // the counts as mismatched, and followed
        // nothing; segment pairing anchors on Ushers,
        // so the renames pair in their segment and the
        // addition lands in its own.
        let mut doc = bare_doc();
        let old_lines: Vec<String> = ["Choir", "Bells", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        doc.profile.serving_lines = old_lines.clone();
        doc.set_participant_value("Choir", "Ann");
        doc.set_participant_value("Bells", "Bob");
        let new_lines: Vec<String> = ["Choir team", "Bells team", "Ushers", "Drivers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Choir team"), "Ann");
        assert_eq!(doc.participant_value_for_name("Bells team"), "Bob");
        assert_eq!(doc.participant_value("choir"), "Ann");
        assert_eq!(doc.participant_value("bells"), "Bob");
    }

    #[test]
    fn a_delete_and_a_rename_in_one_save_report_the_delete_as_removed() {
        // Round 6, RT6-6: Readers renamed to Lectors
        // and Acolytes deleted outright, in one save.
        // The deleted line is reported REMOVED with
        // its names cleared — never "unfollowed" —
        // and the rename pairs in the same save.
        let mut doc = bare_doc();
        let old_lines: Vec<String> = ["Readers", "Ushers", "Acolytes"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        doc.profile.serving_lines = old_lines.clone();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Acolytes", "Kid K");
        let new_lines: Vec<String> = ["Lectors", "Ushers"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(!migration.ambiguous);
        assert_eq!(migration.removed, vec!["Acolytes".to_string()]);
        assert!(!doc.participants.contains_key("acolytes"));
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Lectors"), "Ann");
        assert_eq!(doc.participant_value("readers"), "Ann");
        assert_eq!(doc.participant_value("acolytes"), "");
    }

    #[test]
    fn a_removal_drops_its_aliases_and_a_reused_name_starts_fresh() {
        // Round 6, R6-C: Readers renamed to Lectors
        // records readers to lectors. Removing the
        // Lectors line drops that alias, so the
        // readers token stops resolving; and a NEW
        // line later taking the exact name "Lectors"
        // starts fresh — it inherits neither the
        // alias nor the removed line's names.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        let old_lines = doc.profile.serving_lines.clone();
        let mut renamed = old_lines.clone();
        renamed[0] = "Lectors".to_string();
        migrate_serving_line_renames(
            &old_lines,
            &renamed,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = renamed;
        assert_eq!(doc.participant_value("readers"), "Ann");
        // Remove the renamed line outright.
        let old_lines = doc.profile.serving_lines.clone();
        let mut without = old_lines.clone();
        without.remove(0);
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &without,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(migration.removed, vec!["Lectors".to_string()]);
        assert!(!doc.profile.serving_aliases.contains_key("readers"));
        doc.profile.serving_lines = without;
        assert_eq!(doc.participant_value("readers"), "");
        assert_eq!(doc.participant_value("lectors"), "");
        // A new line reusing the exact name starts
        // fresh: no names, and the old token does not
        // resurrect them onto it.
        let old_lines = doc.profile.serving_lines.clone();
        let mut reused = old_lines.clone();
        reused.insert(0, "Lectors".to_string());
        migrate_serving_line_renames(
            &old_lines,
            &reused,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = reused;
        assert_eq!(doc.participant_value_for_name("Lectors"), "");
        assert_eq!(doc.participant_value("readers"), "");
    }

    #[test]
    fn a_profile_without_the_alias_record_loads_with_none() {
        // Round 6, R6-C: profiles written before the
        // alias record existed carry no
        // `serving_aliases` key and load with an empty
        // record.
        let json = r#"{"name":"Grace","address":"","service_times":"",
            "website":"","giving_url":"","pastor":""}"#;
        let profile: ChurchProfile = serde_json::from_str(json).expect("old profile shape");
        assert!(profile.serving_aliases.is_empty());
        assert_eq!(profile.serving_lines, default_serving_lines());
    }

    #[test]
    fn slug_twins_in_one_list_are_a_conflict() {
        // Round 5, RT5-4 (1): "Prayer Team" and
        // "Prayer-Team" pass exact-name dedupe but
        // derive one slug, so they would share one
        // storage slot.
        let old_lines = default_serving_lines();
        let mut new_lines = old_lines.clone();
        new_lines.push("Prayer Team".to_string());
        new_lines.push("Prayer-Team".to_string());
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &BTreeMap::new())
            .expect("slug twins conflict");
        assert_eq!(conflict.first, "Prayer Team");
        assert_eq!(conflict.second, "Prayer-Team");
        assert_eq!(conflict.slug, "prayer_team");
        assert!(!conflict.builtin);
        // Lookalike but distinct slugs are no
        // conflict: "Prayer Team" and "Prayer Teams"
        // save normally.
        let mut fine_lines = old_lines.clone();
        fine_lines.push("Prayer Team".to_string());
        fine_lines.push("Prayer Teams".to_string());
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &fine_lines, &BTreeMap::new()),
            None
        );
        // A plain reorder of the built-ins is no
        // conflict either.
        let mut swapped = old_lines.clone();
        swapped.swap(0, 5);
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &swapped, &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn a_name_taking_a_builtin_slug_is_a_conflict() {
        // Round 5, RT5-4 (2): an added "Childrens
        // Message" derives the built-in Children's
        // message canonical slug and would hijack its
        // row and its printed token.
        let old_lines = default_serving_lines();
        let mut added = old_lines.clone();
        added.push("Childrens Message".to_string());
        let conflict = serving_line_slug_conflict(&old_lines, &added, &BTreeMap::new())
            .expect("alias addition conflicts");
        assert_eq!(conflict.first, "Childrens Message");
        assert_eq!(conflict.second, "Children's message");
        assert_eq!(conflict.slug, "childrens_message");
        assert!(conflict.builtin);
        // Round 8 (R8-11): the taking line here is a
        // new line, and the conflict says so.
        assert!(conflict.taker_is_new);
        // Round 5, RT5-4 (3): renaming Acolytes onto
        // that name is the same taking, by rename.
        let mut renamed = old_lines.clone();
        renamed[5] = "Childrens Message".to_string();
        let conflict = serving_line_slug_conflict(&old_lines, &renamed, &BTreeMap::new())
            .expect("alias rename conflicts");
        // FLAGGED, round 8 (R8-11): `first` used to be
        // the contested name as typed; the conflict
        // now names the taking line by the name it
        // had — Acolytes — while the holding side
        // keeps the name the bulletin prints for the
        // claimed line.
        assert_eq!(conflict.first, "Acolytes");
        assert_eq!(conflict.second, "Children's message");
        assert!(conflict.builtin);
        assert!(!conflict.taker_is_new);
        // The built-in renaming ITSELF to the same
        // words without the apostrophe is the line
        // itself: no conflict, and after the save its
        // row and its token read the same names.
        let mut itself = old_lines.clone();
        itself[4] = "Childrens Message".to_string();
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &itself, &BTreeMap::new()),
            None
        );
        let mut doc = bare_doc();
        doc.profile.serving_lines = old_lines.clone();
        doc.set_participant_value("Children's message", "Ms. Patty");
        migrate_serving_line_renames(
            &old_lines,
            &itself,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        doc.profile.serving_lines = itself;
        assert_eq!(
            doc.participant_value_for_name("Childrens Message"),
            "Ms. Patty"
        );
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Patty"
        );
        // The built-in under its own name is itself.
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &default_serving_lines(), &BTreeMap::new()),
            None
        );
    }

    #[test]
    fn a_claimant_renaming_back_inside_an_ambiguous_edit_is_not_its_own_conflict() {
        // Round 7 (review m1, R7-9): the old list's
        // "Kids message" claims childrens_message
        // through the recorded alias. The new list
        // renames it back to "Children's message" and
        // also renames and moves Readers and Acolytes
        // across anchors, so the pairing is ambiguous
        // and its pairs are discarded. The conflict
        // check used to lose the claimant's self-excuse
        // with those pairs and report the line as
        // conflicting with itself ("'Children's
        // message' would take the place of the built-in
        // line 'Children's message'..."). The verdict
        // for this edit is now NOT a conflict at all:
        // the save is still refused, by the ambiguous
        // path, whose kinds name the renames and the
        // crossing that actually block it.
        let old_lines: Vec<String> = [
            "Readers",
            "Ushers",
            "Greeters",
            "Communion assistants",
            "Kids message",
            "Acolytes",
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("childrens_message".to_string(), "kids_message".to_string());
        let new_lines: Vec<String> = [
            "Candle lighters",
            "Lectors",
            "Ushers",
            "Greeters",
            "Communion assistants",
            "Children's message",
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();
        assert_eq!(
            serving_line_slug_conflict(&old_lines, &new_lines, &aliases),
            None,
            "the claimant renaming back to its built-in name is not a self-conflict"
        );
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        let mut alias_record = aliases.clone();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut alias_record,
        );
        assert!(
            migration.ambiguous,
            "the save is still refused, as ambiguous"
        );
        assert_eq!(
            migration.ambiguous_kinds,
            Some(ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            })
        );
        // The standing pin beside it: a DIFFERENT line
        // taking the built-in's name in the same kind
        // of edit is still a hijack and still fires.
        let hijack: Vec<String> = [
            "Lectors",
            "Ushers",
            "Greeters",
            "Communion assistants",
            "Kids message",
            "Children's message",
        ]
        .iter()
        .map(|n| n.to_string())
        .collect();
        let conflict = serving_line_slug_conflict(&old_lines, &hijack, &aliases)
            .expect("a different line taking the name still conflicts");
        assert!(conflict.builtin);
        // FLAGGED, round 8 (R8-11): `first` used to be
        // the contested name itself; the conflict now
        // names the taking line by the name it had
        // (Acolytes, paired with the name inside its
        // segment) and the holding side by the name
        // the bulletin currently prints for the
        // claimed line ("Kids message").
        assert_eq!(conflict.first, "Acolytes");
        assert_eq!(conflict.second, "Kids message");
        assert_eq!(conflict.slug, "children_s_message");
    }

    #[test]
    fn ambiguous_kinds_count_only_segments_the_pairing_could_not_resolve() {
        // Round 8 (R8-10, all three lanes): the
        // `moved` tally used to increment for
        // removal-only and addition-only segments
        // too, so an ambiguous rename segment plus a
        // clean removal elsewhere claimed a move that
        // never happened, and the addition in the
        // first shape below was omitted while the
        // absent move was claimed. Resolved segments
        // now contribute their own kinds only.
        let kinds_of = |old: &[&str], new: &[&str]| {
            let old_lines: Vec<String> = old.iter().map(|n| n.to_string()).collect();
            let new_lines: Vec<String> = new.iter().map(|n| n.to_string()).collect();
            let mut participants: BTreeMap<String, String> = BTreeMap::new();
            let mut aliases: BTreeMap<String, String> = BTreeMap::new();
            let migration = migrate_serving_line_renames_detailed(
                &old_lines,
                &new_lines,
                &mut participants,
                &mut aliases,
            );
            assert!(migration.ambiguous, "the refusal itself stands");
            migration.ambiguous_kinds.expect("kinds on a refusal")
        };
        // The red team's S-C: one rename segment, one
        // clean removal, one clean addition; nothing
        // crossed an anchor. FLAGGED, round 10
        // (R10-8): the rename segment is a deficit
        // segment, and its unpaired old name's only
        // possible partner is the trailing addition —
        // the mirror of the middle-segment witness in
        // `r9_kinds_come_from_each_segments_own_outcome`.
        // The deficit now answers that addition
        // across the anchor (a move, not an addition
        // and a second removal); the clean removal in
        // its own segment, which no surplus can
        // absorb, still stands as a removal.
        assert_eq!(
            kinds_of(
                &["Alpha", "Old One", "Old Two", "Beta", "Gone Line", "Gamma"],
                &["Alpha", "New One", "Beta", "Gamma", "Fresh Line"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: false,
                moved: true,
            }
        );
        // The red team's S-D: rename segment plus a
        // removal only — no move, and no addition.
        assert_eq!(
            kinds_of(
                &["Alpha", "Old One", "Old Two", "Beta", "Gone", "Gamma"],
                &["Alpha", "New One", "Beta", "Gamma"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: false,
                moved: false,
            }
        );
        // The verify lane's D8 shape: an ambiguous
        // rename segment between two anchors, plus a
        // line removed in its own segment after the
        // last anchor.
        assert_eq!(
            kinds_of(
                &["Alpha", "Choir", "Bells", "Beta", "Solo"],
                &["Alpha", "Choir team", "Beta"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: false,
                moved: false,
            }
        );
        // The canonical crossed-renames shape keeps
        // its kinds exactly: two renames crossing
        // anchors, nothing removed or added. (The
        // Acolytes line's old-only segment counts
        // toward `moved` here because the first
        // segment holds more new names than old ones:
        // one of those names' partners — the Acolytes
        // line itself — must sit in another segment.)
        assert_eq!(
            kinds_of(
                &[
                    "Readers",
                    "Ushers",
                    "Greeters",
                    "Communion assistants",
                    "Children's message",
                    "Acolytes",
                ],
                &[
                    "Candle lighters",
                    "Lectors",
                    "Ushers",
                    "Greeters",
                    "Communion assistants",
                    "Children's message",
                ],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            }
        );
    }

    #[test]
    fn r9_kinds_come_from_each_segments_own_outcome() {
        // Round 9 (R9-14, review R9-R3 + red team
        // RT9-14): the kinds used to be read off the
        // two lists' global totals plus a surplus
        // inference — an unresolved segment holding
        // surplus new names made every old-only
        // segment count toward `moved` and suppressed
        // `removed`. Two mirror compositions broke in
        // opposite directions, and both are pinned
        // here with the kind sets their segments' own
        // outcomes show.
        let kinds_of = |old: &[&str], new: &[&str]| {
            let old_lines: Vec<String> = old.iter().map(|n| n.to_string()).collect();
            let new_lines: Vec<String> = new.iter().map(|n| n.to_string()).collect();
            let mut participants: BTreeMap<String, String> = BTreeMap::new();
            let mut aliases: BTreeMap<String, String> = BTreeMap::new();
            let migration = migrate_serving_line_renames_detailed(
                &old_lines,
                &new_lines,
                &mut participants,
                &mut aliases,
            );
            assert!(migration.ambiguous, "the refusal itself stands");
            migration.ambiguous_kinds.expect("kinds on a refusal")
        };
        // The review lane's K1: a deficit rename
        // segment (two old names against one new)
        // plus an addition-only segment. Round 9
        // pinned the segment-local reading {renamed,
        // removed, added}. FLAGGED, round 10 (R10-8):
        // this shape is the trailing mirror of the
        // canonical crossed renames below — a deficit
        // plus a trailing addition-only segment —
        // and the mirror reads {renamed, moved}
        // there, so the two geometries disagreed
        // about the same shape. The deficit now
        // answers the addition-only segment's name
        // across the anchor, as a surplus answers a
        // removal-only segment's name, and both
        // mirrors read {renamed, moved}.
        assert_eq!(
            kinds_of(
                &["Alpha", "Old One", "Old Two", "Beta"],
                &["Alpha", "New One", "Beta", "Fresh Line"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            }
        );
        // The red team's RT9-14 witness: a surplus
        // rename segment plus a removal-only segment
        // BETWEEN anchors. Round 9 pinned {renamed,
        // removed, added}: the bounded segment's
        // removal read as resolved in place.
        // FLAGGED, round 10 (R10-8): a pure crossed
        // reading exists here too (the bounded
        // segment's name is the surplus name's
        // partner, moved across the anchor), so the
        // removal was asserted past what the pairing
        // establishes. A standing surplus now
        // absorbs a removal-only segment's names by
        // count anywhere in the list, not only in
        // the trailing segment, and the shape reads
        // as the crossed renames it may be.
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "X2", "Beta", "Gone", "Gamma"],
                &["Alpha", "Y1", "Y2", "Y3", "Beta", "Gamma"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            }
        );
        // The same shape plus a clean addition-only
        // segment: the surplus is spent on the
        // bounded segment's name, so the trailing
        // addition still stands as an addition.
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "X2", "Beta", "Gone", "Gamma"],
                &["Alpha", "Y1", "Y2", "Y3", "Beta", "Gamma", "Fresh"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: true,
                moved: true,
            }
        );
        // The mirror control (review K2): a surplus
        // segment plus an old-only segment AFTER the
        // last anchor — the canonical crossed-renames
        // geometry, where the trailing segment's name
        // is the surplus name's partner across the
        // anchor. {renamed, moved}, as pinned since
        // round 6.
        assert_eq!(
            kinds_of(
                &["Alpha", "Old One", "Beta", "Gone"],
                &["Alpha", "New One", "New Two", "Beta"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            }
        );
        // A surplus segment standing alone: the
        // surplus name has no partner segment to sit
        // in, so it reads as an addition and no move
        // is claimed (the red team's control).
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "X2", "Beta"],
                &["Alpha", "Y1", "Y2", "Y3", "Beta"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: true,
                moved: false,
            }
        );
    }

    #[test]
    fn r10_kinds_claim_only_what_the_pairing_establishes() {
        // Round 10 (R10-8, red team RT10-K1 + RT10-K2
        // and the review lane's Q3 observation): the
        // kinds asserted a plain removal (and a plain
        // addition) where a standing surplus (or
        // deficit) could have absorbed the partner
        // segment's names across an anchor, and the
        // two trailing mirror geometries read by
        // opposite rules. The reservoir is now
        // counted by name anywhere in the list, in
        // both directions; mirror geometries produce
        // the same kinds, and the shapes the stated
        // rule names (both round-10 lanes re-derived
        // them) read exactly as before.
        let kinds_of = |old: &[&str], new: &[&str]| {
            let old_lines: Vec<String> = old.iter().map(|n| n.to_string()).collect();
            let new_lines: Vec<String> = new.iter().map(|n| n.to_string()).collect();
            let mut participants: BTreeMap<String, String> = BTreeMap::new();
            let mut aliases: BTreeMap<String, String> = BTreeMap::new();
            let migration = migrate_serving_line_renames_detailed(
                &old_lines,
                &new_lines,
                &mut participants,
                &mut aliases,
            );
            assert!(migration.ambiguous, "the refusal itself stands");
            migration.ambiguous_kinds.expect("kinds on a refusal")
        };
        let crossed = ServingLineEditKinds {
            renamed: true,
            removed: false,
            added: false,
            moved: true,
        };
        // The middle-segment witness (RT10-K1): a
        // surplus segment whose only possible partner
        // sits in a MIDDLE removal-only segment. A
        // pure crossed-rename reading exists, so no
        // addition and no removal is asserted.
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "Beta", "Gone", "Gamma"],
                &["Alpha", "Y1", "Y2", "Beta", "Gamma"],
            ),
            crossed
        );
        // Its mirror: a deficit segment whose only
        // possible partner sits in a MIDDLE
        // addition-only segment. Identical kinds.
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "X2", "Beta", "Gamma"],
                &["Alpha", "Y1", "Beta", "Fresh", "Gamma"],
            ),
            crossed
        );
        // The trailing mirrors (RT10-K2): deficit
        // plus a trailing addition-only segment, and
        // surplus plus a trailing removal-only
        // segment — the same geometry, and now the
        // same account.
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "X2", "Beta"],
                &["Alpha", "Y1", "Beta", "Fresh"],
            ),
            crossed
        );
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "Beta", "Gone"],
                &["Alpha", "Y1", "Y2", "Beta"],
            ),
            crossed
        );
        // The certain shapes, verbatim: a surplus of
        // one against three trailing old names — one
        // partner used, two past the need are
        // removals.
        let one_partner_two_removals = ServingLineEditKinds {
            renamed: true,
            removed: true,
            added: false,
            moved: true,
        };
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "Beta", "Gone1", "Gone2", "Gone3"],
                &["Alpha", "Y1", "Y2", "Beta"],
            ),
            one_partner_two_removals
        );
        // A surplus of two against the same three.
        assert_eq!(
            kinds_of(
                &["Alpha", "X1", "Beta", "Gone1", "Gone2", "Gone3"],
                &["Alpha", "Y1", "Y2", "Y3", "Beta"],
            ),
            one_partner_two_removals
        );
        // A surplus in the FIRST segment, its partner
        // in the trailing segment, and an additional
        // bounded removal no surplus is left to
        // absorb: the removal stands.
        assert_eq!(
            kinds_of(
                &["X1", "Alpha", "Gone", "Beta", "Lost"],
                &["Y1", "Y2", "Alpha", "Beta"],
            ),
            one_partner_two_removals
        );
        // A standing removal no surplus can absorb
        // (there is no surplus at all) stays a
        // removal, and no move is claimed.
        assert_eq!(
            kinds_of(
                &["Alpha", "Choir", "Bells", "Beta", "Solo"],
                &["Alpha", "Choir team", "Beta"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: true,
                added: false,
                moved: false,
            }
        );
    }

    #[test]
    fn r11d_duplicate_names_spend_each_occurrence_once_in_the_kinds() {
        // Round 11 (R11-D, red team RT11-8): the
        // reservoir accounting keyed on names, so a
        // name standing in two segments was spent
        // twice — paired in one segment and absorbed
        // by the reservoir from another — and a real
        // removal was hidden behind a phantom move.
        // The accounting is per occurrence now: an
        // occurrence whose name a pairing already
        // spent cannot also feed the reservoir.
        let kinds_of = |old: &[&str], new: &[&str]| {
            let old_lines: Vec<String> = old.iter().map(|n| n.to_string()).collect();
            let new_lines: Vec<String> = new.iter().map(|n| n.to_string()).collect();
            let mut participants: BTreeMap<String, String> = BTreeMap::new();
            let mut aliases: BTreeMap<String, String> = BTreeMap::new();
            let migration = migrate_serving_line_renames_detailed(
                &old_lines,
                &new_lines,
                &mut participants,
                &mut aliases,
            );
            assert!(migration.ambiguous, "the refusal itself stands");
            migration.ambiguous_kinds.expect("kinds on a refusal")
        };
        // The RT11-8 witness: Papa is paired
        // Papa→Quebec in the first segment and
        // removal-only in the second; Uniform's
        // segment holds the surplus. The second
        // Papa's removal was absorbed as the
        // surplus's cross-anchor partner and the
        // kinds read {renamed, moved}. The removal
        // now stands, the surplus the reservoir
        // could not spend reads as an addition, and
        // no move is claimed.
        let duplicate_kinds = ServingLineEditKinds {
            renamed: true,
            removed: true,
            added: true,
            moved: false,
        };
        assert_eq!(
            kinds_of(
                &["Alpha", "Papa", "Beta", "Papa", "Gamma", "Uniform", "Delta"],
                &["Alpha", "Quebec", "Beta", "Gamma", "Victor", "Whiskey", "Delta"],
            ),
            duplicate_kinds
        );
        // The deficit-side mirror (duplicate NEW
        // names): Papa is the first segment's rename
        // target and addition-only in the second;
        // the deficit segment's leftover old name
        // could previously pair across the anchor
        // with that already-spent Papa. Identical
        // kinds.
        assert_eq!(
            kinds_of(
                &["Alpha", "Quebec", "Beta", "Gamma", "Victor", "Whiskey", "Delta"],
                &["Alpha", "Papa", "Beta", "Papa", "Gamma", "Uniform", "Delta"],
            ),
            duplicate_kinds
        );
        // Control: the same geometry with a UNIQUE
        // removal-only name still reads as the
        // canonical crossed renames — the reservoir
        // absorbs the unspent occurrence exactly as
        // before (the R10-8 middle-segment witness).
        assert_eq!(
            kinds_of(
                &["Alpha", "Papa", "Beta", "Gone", "Gamma", "Uniform", "Delta"],
                &["Alpha", "Quebec", "Beta", "Gamma", "Victor", "Whiskey", "Delta"],
            ),
            ServingLineEditKinds {
                renamed: true,
                removed: false,
                added: false,
                moved: true,
            }
        );
    }

    #[test]
    fn r11d_duplicate_pairings_keep_participants_and_aliases_in_step() {
        // Round 11 (R11-D, red team RT11-8): with a
        // duplicate name, the migration followed
        // every pairing and removal the segments
        // computed, but the stored names sit under
        // one slug and the alias under one key, so
        // the follow-through desynced: a paired
        // name's removal occurrence erased the alias
        // the pairing had just written, and a double
        // pairing left the people on the first target
        // while the surviving alias pointed at the
        // second. Each old name is now spent once:
        // the first pairing in segment order is the
        // pairing actually made, and the recorded
        // aliases correspond one-to-one with it.
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        // Paired in one segment, removal-only in
        // another: the people move with the pairing,
        // the alias the pairing wrote survives, and
        // nothing is reported removed — the name's
        // stored value was renamed, not removed.
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Xray", "Gamma"]);
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert!(!participants.contains_key("xray"));
        assert_eq!(aliases.get("xray").map(String::as_str), Some("yankee"));
        assert_eq!(aliases.len(), 1);
        // Paired twice, to two different targets:
        // the people land on the first target and
        // the surviving alias points at that same
        // target — one pairing made, one alias
        // recorded, in step.
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Zulu", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert!(!participants.contains_key("zulu"));
        assert!(!participants.contains_key("xray"));
        assert_eq!(aliases.get("xray").map(String::as_str), Some("yankee"));
        assert_eq!(aliases.len(), 1);
        // Duplicates on the NEW side: two different
        // old names paired to the same duplicated
        // new name. Both pairings are made (their
        // old names are distinct), both aliases are
        // recorded, and the target-conflict rule
        // holds — the first value lands, the second
        // stays where it was rather than being
        // discarded.
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Zulu", "Gamma"]);
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Yankee", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        participants.insert("zulu".to_string(), "Zoe".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert_eq!(participants.get("zulu").map(String::as_str), Some("Zoe"));
        assert_eq!(aliases.get("xray").map(String::as_str), Some("yankee"));
        assert_eq!(aliases.get("zulu").map(String::as_str), Some("yankee"));
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn r12c_unfollowed_pairings_and_removals_are_carried_on_the_migration() {
        // Round 12 (R12-11, review R12-8): the R11-D
        // accounting stands — the first pairing in
        // segment order is the one made, and a later
        // pairing of the same old name, or a removal
        // of it, is not followed — but the events not
        // followed are no longer silent: the migration
        // carries each one, naming the old name, the
        // pairing that was kept, and the pairing or
        // removal that was not followed, so the app's
        // status can speak and the office can correct
        // the names by hand.
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Xray", "Gamma"]);
        // Paired twice, to two different targets: the
        // second pairing is carried as not followed,
        // and the accounting is the R11-D pin's own —
        // the people and the alias stand with the
        // first target.
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Zulu", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert!(!participants.contains_key("zulu"));
        assert_eq!(aliases.get("xray").map(String::as_str), Some("yankee"));
        let noted_old_name = "Xray".to_string();
        let noted_kept_new_name = "Yankee".to_string();
        let noted_unfollowed_new_name = Some("Zulu".to_string());
        let expected_pairing = ServingLineUnfollowed {
            old_name: noted_old_name,
            kept_new_name: noted_kept_new_name,
            unfollowed_new_name: noted_unfollowed_new_name,
        };
        assert_eq!(migration.unfollowed, vec![expected_pairing]);
        // Paired in one segment, removal-only in
        // another: the removal is carried as not
        // followed, and again nothing is reported
        // removed — the name's stored value was
        // renamed, not removed.
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert_eq!(aliases.get("xray").map(String::as_str), Some("yankee"));
        let noted_old_name = "Xray".to_string();
        let noted_kept_new_name = "Yankee".to_string();
        let expected_removal = ServingLineUnfollowed {
            old_name: noted_old_name,
            kept_new_name: noted_kept_new_name,
            unfollowed_new_name: None,
        };
        assert_eq!(migration.unfollowed, vec![expected_removal]);
        // No duplicates: a plain rename, and a plain
        // removal beside it, are both followed and
        // carry no unfollowed note.
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Gone", "Gamma"]);
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        participants.insert("gone".to_string(), "Gus".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert_eq!(migration.removed, vec!["Gone".to_string()]);
        assert!(migration.unfollowed.is_empty());
    }

    #[test]
    fn r13c_same_target_pairings_are_not_unfollowed_events() {
        // Round 13 (R13-7a, review R13-6 + red team
        // RT13-6a): a duplicate pairing to the SAME
        // target produced an unfollowed event whose
        // sentence was a tautological false alarm —
        // "'Xray' was paired with 'Yankee' and with
        // 'Yankee' … the pairing with 'Yankee' was
        // not followed. Correct the names by hand" —
        // although the end state is byte-identical
        // to the single pairing and nothing was
        // lost. The event is not recorded now; the
        // accounting (the name is spent once, the
        // names and the alias stand with the kept
        // pairing) is unchanged.
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Xray", "Gamma"]);
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Yankee", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert!(
            migration.unfollowed.is_empty(),
            "a same-target duplicate pairing carries no unfollowed event"
        );
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert_eq!(aliases.get("xray").map(String::as_str), Some("yankee"));
        // A triple pairing — Yankee, Zulu, Yankee —
        // carries only the real Zulu event; the
        // same-target repeat beside it does not
        // dilute it with a false alarm.
        let old_lines = lines(&["Alpha", "Xray", "Beta", "Xray", "Gamma", "Xray", "Delta"]);
        let new_lines = lines(&[
            "Alpha", "Yankee", "Beta", "Zulu", "Gamma", "Yankee", "Delta",
        ]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        let noted_old_name = "Xray".to_string();
        let noted_kept_new_name = "Yankee".to_string();
        let noted_unfollowed_new_name = Some("Zulu".to_string());
        let expected = ServingLineUnfollowed {
            old_name: noted_old_name,
            kept_new_name: noted_kept_new_name,
            unfollowed_new_name: noted_unfollowed_new_name,
        };
        assert_eq!(migration.unfollowed, vec![expected]);
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert!(!participants.contains_key("zulu"));
        // A different target still notes, as R12-C
        // pinned, and an unfollowed REMOVAL is
        // unaffected: a removal is never the same
        // event as the kept pairing, even when the
        // same name's duplicate pairing to its kept
        // target stands in the same edit. (Covered
        // by the segment-order pin below, whose
        // first construction carries a removal.)
        let new_lines = lines(&["Alpha", "Yankee", "Beta", "Zulu", "Gamma"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &lines(&["Alpha", "Xray", "Beta", "Xray", "Gamma"]),
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert_eq!(migration.unfollowed.len(), 1);
        assert_eq!(
            migration.unfollowed[0].unfollowed_new_name.as_deref(),
            Some("Zulu")
        );
    }

    #[test]
    fn r13c_unfollowed_events_are_carried_in_segment_order() {
        // Round 13 (R13-7b, red team RT13-6b):
        // unfollowed events were collected
        // pairings-then-removals, so in a
        // pair/removal/pair edit the status named
        // the later pairing before the removal that
        // precedes it in the edit. The events are
        // carried in segment order now — the order
        // the office wrote them — with the
        // accounting unchanged.
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        // Pair, removal, pair: Xray pairs to Yankee
        // (kept), stands removal-only in the next
        // segment, then pairs to Zulu.
        let old_lines = lines(&["A0", "Xray", "A1", "Xray", "A2", "Xray", "A3"]);
        let new_lines = lines(&["A0", "Yankee", "A1", "A2", "Zulu", "A3"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert!(migration.removed.is_empty());
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert_eq!(aliases.get("xray").map(String::as_str), Some("yankee"));
        let removal_old = "Xray".to_string();
        let removal_kept = "Yankee".to_string();
        let expected_removal = ServingLineUnfollowed {
            old_name: removal_old,
            kept_new_name: removal_kept,
            unfollowed_new_name: None,
        };
        let pairing_old = "Xray".to_string();
        let pairing_kept = "Yankee".to_string();
        let pairing_unfollowed = Some("Zulu".to_string());
        let expected_pairing = ServingLineUnfollowed {
            old_name: pairing_old,
            kept_new_name: pairing_kept,
            unfollowed_new_name: pairing_unfollowed,
        };
        assert_eq!(
            migration.unfollowed,
            vec![expected_removal, expected_pairing],
            "the removal was written before the Zulu pairing, and is carried first"
        );
        // Two names interleaved: a removal of Xray
        // in segment two precedes an unfollowed
        // pairing of Yoke in segment four, and the
        // carried order follows the segments, not
        // the kinds.
        let old_lines = lines(&["A0", "Xray", "A1", "Xray", "A2", "Yoke", "A3", "Yoke", "A4"]);
        let new_lines = lines(&["A0", "Yankee", "A1", "A2", "Zed", "A3", "Whiskey", "A4"]);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        participants.insert("xray".to_string(), "Ann".to_string());
        participants.insert("yoke".to_string(), "Yolanda".to_string());
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut aliases,
        );
        assert!(!migration.ambiguous);
        assert_eq!(participants.get("yankee").map(String::as_str), Some("Ann"));
        assert_eq!(participants.get("zed").map(String::as_str), Some("Yolanda"));
        let first_old = "Xray".to_string();
        let first_kept = "Yankee".to_string();
        let expected_first = ServingLineUnfollowed {
            old_name: first_old,
            kept_new_name: first_kept,
            unfollowed_new_name: None,
        };
        let second_old = "Yoke".to_string();
        let second_kept = "Zed".to_string();
        let second_unfollowed = Some("Whiskey".to_string());
        let expected_second = ServingLineUnfollowed {
            old_name: second_old,
            kept_new_name: second_kept,
            unfollowed_new_name: second_unfollowed,
        };
        assert_eq!(migration.unfollowed, vec![expected_first, expected_second]);
    }

    #[test]
    fn r9_positional_evidence_reads_the_segments_unfiltered_runs() {
        // Round 9 (R9-15, red team RT9-15): the
        // positional excuse read the pairing's segment
        // runs AFTER kept names were filtered out, so
        // a kept name moved into the segment shifted
        // every later position. The Beta/Wanderer
        // witness, verbatim: "Kids message" claims the
        // children's-message slug through the alias;
        // in the contested segment the lines stand
        // old [Kids message, Choir] against new [Beta,
        // Children's message, New B, Extra] — Beta is
        // a kept line that moved into the segment. On
        // the unfiltered runs, "Children's message"
        // stands at Choir's position (1), and the
        // conflict names Choir as the taker. On the
        // filtered runs it stood at index 0, aligned
        // with the claimant, and the hijack was
        // excused with its reason lost (the save was
        // still refused, as ambiguous).
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("childrens_message".to_string(), "kids_message".to_string());
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        let old_lines = lines(&[
            "Alpha",
            "Kids message",
            "Choir",
            "Wanderer",
            "Beta",
            "Omega",
        ]);
        let new_lines = lines(&[
            "Alpha",
            "Beta",
            "Children's message",
            "New B",
            "Extra",
            "Wanderer",
            "Omega",
        ]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the hijack's reason is named, not excused");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Choir");
        assert_eq!(conflict.second, "Kids message");
        assert!(!conflict.taker_is_new);
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        let mut alias_record = aliases.clone();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut alias_record,
        );
        assert!(migration.ambiguous, "the save is still refused");
    }

    #[test]
    fn a_slug_conflict_names_the_taking_line_and_the_current_holder() {
        // Round 8 (R8-11, review R8-R8 + red team
        // RT8-7/RT8-8): around the built-in's own
        // name, the conflict used to name the built-in
        // on both sides, excuse a hijack that merely
        // shared the unpaired sets, and miss a
        // claimant the pairing guessed across inside
        // an equal-count segment. The judgment now
        // works from the pairing's best evidence. In
        // every shape here "Kids message" claims
        // childrens_message through the recorded
        // alias, so the holding side is "Kids
        // message" — the name the bulletin currently
        // prints for that line.
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("childrens_message".to_string(), "kids_message".to_string());
        let lines =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        // S-A2: the segment pairs in order — Kids
        // message to "Story time", Choir to
        // "Children's message". Choir's line is the
        // taker, named by the name it had.
        let old_lines = lines(&["Readers", "Kids message", "Choir", "Ushers"]);
        let new_lines = lines(&["Readers", "Story time", "Children's message", "Ushers"]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("choir taking the claimed name conflicts");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Choir");
        assert_eq!(conflict.second, "Kids message");
        assert!(!conflict.taker_is_new);
        // S-B: the claimant renames back while another
        // (new) line takes its freed name in the same
        // save. The rename-back line is not accused of
        // conflicting with itself: the taker is a new
        // line, and the holder is "Kids message", so
        // no side names the contested name
        // unqualified.
        let old_lines = lines(&["Readers", "Kids message", "Ushers"]);
        let new_lines = lines(&["Readers", "Children's message", "Kids message", "Ushers"]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the new line taking the claimed name conflicts");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Children's message");
        assert_eq!(conflict.second, "Kids message");
        assert!(conflict.taker_is_new);
        // S-A: a genuine hijack sharing the unpaired
        // sets. The segment holds old [Kids message,
        // Choir] against new [Story time, Children's
        // message, Bonus line]: the claimant's own
        // in-segment position pairs it with "Story
        // time", and Choir's line sits at the
        // contested name's position. The membership
        // excuse used to excuse this; the conflict
        // now names the hijack, and the save is still
        // refused (by the migration, as ambiguous).
        let old_lines = lines(&["Readers", "Kids message", "Choir", "Ushers"]);
        let new_lines = lines(&[
            "Readers",
            "Story time",
            "Children's message",
            "Bonus line",
            "Ushers",
        ]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the hijack is named, not excused");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Choir");
        assert_eq!(conflict.second, "Kids message");
        let mut participants: BTreeMap<String, String> = BTreeMap::new();
        let mut alias_record = aliases.clone();
        let migration = migrate_serving_line_renames_detailed(
            &old_lines,
            &new_lines,
            &mut participants,
            &mut alias_record,
        );
        assert!(migration.ambiguous, "the save is still refused");
        // The review's crossed-segment shape: inside
        // an equal-count segment the pairing pairs
        // Kids message with "Other name" and Other
        // line with "Children's message", while a
        // different segment is ambiguous. Membership
        // found the claimant in neither a pair to the
        // name nor the leftover sets and printed the
        // self-conflict sentence; the positional
        // evidence reports the conflict against Other
        // line instead.
        let old_lines = lines(&[
            "Alpha",
            "Kids message",
            "Other line",
            "Beta",
            "Old One",
            "Old Two",
            "Gamma",
        ]);
        let new_lines = lines(&[
            "Alpha",
            "Other name",
            "Children's message",
            "Beta",
            "New One",
            "Gamma",
        ]);
        let conflict = serving_line_slug_conflict(&old_lines, &new_lines, &aliases)
            .expect("the crossed segment's taker is named");
        assert!(conflict.builtin);
        assert_eq!(conflict.first, "Other line");
        assert_eq!(conflict.second, "Kids message");
    }

    #[test]
    fn a_removed_line_loses_its_names_and_is_reported() {
        // Round 4, RT-6c: deleting a line from the
        // profile list used to leave its names in the
        // map, invisible in every list yet still
        // printing through the line's token. The save
        // now clears them, and the migration returns
        // the line's name so the app can say so.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ana Ruiz");
        doc.set_participant_value("Acolytes", "Sam, Jo");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines.remove(5);
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(removed, vec!["Acolytes".to_string()]);
        assert!(!doc.participants.contains_key("acolytes"));
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value("acolytes"), "");
        assert_eq!(resolve_fields("{Sunday.acolytes}", &doc), "");
        assert_eq!(
            doc.participants.get("readers").map(String::as_str),
            Some("Ana Ruiz")
        );
        // A line removed while it holds no names is
        // not reported: nothing was removed.
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines.retain(|n| n != "Greeters");
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert!(removed.is_empty());
    }

    #[test]
    fn a_rename_onto_an_existing_line_clears_the_merged_names() {
        // Round 4, RT-6b: renaming "Readers" onto the
        // already-present "Ushers" leaves Ushers' own
        // names in place, and the merged-away line's
        // names are cleared and reported instead of
        // vanishing silently.
        let mut doc = bare_doc();
        doc.profile.serving_lines = default_serving_lines();
        doc.set_participant_value("Readers", "Ann");
        doc.set_participant_value("Ushers", "Uma");
        let old_lines = doc.profile.serving_lines.clone();
        let mut new_lines = old_lines.clone();
        new_lines[0] = "Ushers".to_string();
        let removed = migrate_serving_line_renames(
            &old_lines,
            &new_lines,
            &mut doc.participants,
            &mut doc.profile.serving_aliases,
        );
        assert_eq!(removed, vec!["Readers".to_string()]);
        assert!(!doc.participants.contains_key("readers"));
        assert_eq!(
            doc.participants.get("ushers").map(String::as_str),
            Some("Uma")
        );
        doc.profile.serving_lines = new_lines;
        assert_eq!(doc.participant_value_for_name("Ushers"), "Uma");
    }

    #[test]
    fn childrens_message_stores_under_its_shown_slug_and_resolves_by_its_token() {
        // The shown name derives "children_s_message"; the
        // permanent token is {Sunday.childrens_message}.
        // The alias table bridges them.
        let mut doc = bare_doc();
        doc.set_participant_value("Children's message", "Ms. Dee");
        assert_eq!(
            doc.participants
                .get("children_s_message")
                .map(String::as_str),
            Some("Ms. Dee")
        );
        assert_eq!(
            resolve_fields("{Sunday.childrens_message}", &doc),
            "Ms. Dee"
        );
        assert_eq!(
            doc.participant_value_for_name("Children's message"),
            "Ms. Dee"
        );
    }
}
