//! Asset rights and license judgment: where a picture
//! or music file came from, what credit it is owed, and
//! how the records for the same bytes merge without
//! manufacturing a permission no one asserted. The
//! document model types these records ride on stay in
//! the crate root; this module is the engine that
//! judges and folds them.

use serde::{Deserialize, Serialize};

/// What an asset in the document's store is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// A picture placed (or waiting to be placed) in the
    /// bulletin: cover art, a photo, clipart.
    Picture,
    /// A tracing-guide picture under a panel, from an import
    /// rescue. Shown while editing, never printed.
    TracingPicture,
    /// A music file carried with the bulletin (MusicXML and
    /// friends; the music build gives it frames).
    Music,
    Other,
}

/// The license an asset's rights record claims, in the
/// vocabulary of docs/research/clipart-landscape.md section 5.
/// The record is a claim the office asserts, never a proof:
/// the app records, composes credit lines, and reminds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseKind {
    PublicDomain,
    Cc0,
    /// Creative Commons Attribution, with its version ("4.0").
    CcBy(String),
    /// Creative Commons Attribution-ShareAlike, with version.
    CcBySa(String),
    /// The church's own work, or a work it holds outright.
    ChurchOwned,
    /// Reprinted under a church license (CCLI, One License).
    LicensedReprint,
    #[serde(other)]
    #[default]
    Unknown,
}

/// Where an asset came from and what credit it is owed
/// (clipart-landscape.md section 5). Every field defaults, so
/// a partial record loads and simply says less.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightsRecord {
    /// Where it came from, in words ("Openclipart", "the
    /// church camera", the imported file it rode in on).
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub license: LicenseKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub creator: String,
    /// The copyright line as it should print, when one is
    /// owed ("\u{00a9} 2019 Jane Doe").
    #[serde(default)]
    pub copyright_line: String,
    /// The exact credit text the license requires, when it
    /// requires one.
    #[serde(default)]
    pub credit_text: String,
    /// The licensor's reference: a CCLI or One License
    /// number, a catalog id, a permission letter's date.
    #[serde(default)]
    pub licensor_ref: String,
    /// Who in the office asserted this record.
    #[serde(default)]
    pub asserted_by: String,
    /// When they asserted it (ISO date).
    #[serde(default)]
    pub asserted_at: String,
    /// Are there children in this picture, as far as the
    /// office knows (roles build R-2b)? None means nobody
    /// has said yet, and Check asks. Some(true) puts the
    /// picture under the web page's stricter rule: it
    /// rides the web page only when a permission is
    /// noted on this record. Print never drops a picture
    /// over this. Like the whole record, it is the
    /// office's own note, not a proof.
    #[serde(default)]
    pub children_present: Option<bool>,
}

/// One asset in the document's store. Its bytes live in
/// `Document::asset_bytes` under the same hash and travel as
/// the zip entry `assets/<sha256>` in a v1 file; the metadata
/// (including the rights record) rides in document.json.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub sha256: String,
    pub kind: AssetKind,
    /// IANA media type, e.g. "image/png".
    pub media_type: String,
    #[serde(default)]
    pub rights: RightsRecord,
}

/// Is a permission noted on this record, for the web
/// page's children rule? This is the same test the HTML
/// export applies before a picture with children noted
/// in it may ride the web page
/// (crates/ob-extract/src/html_export.rs,
/// `permission_noted`, whose copy points back here):
/// a credit line counts, and so does a license that
/// needs no one's permission (the church's own work,
/// public domain, CC0). The two copies are one rule
/// and must stay one rule: the export holds pictures
/// back by it, and the merge below enforces its
/// no-manufactured-permission property by it (round 5,
/// F2; round 6, R6-1).
fn rights_permission_noted(rights: &RightsRecord) -> bool {
    !rights.credit_text.trim().is_empty()
        || matches!(
            rights.license,
            LicenseKind::ChurchOwned | LicenseKind::PublicDomain | LicenseKind::Cc0
        )
}

/// Merge two records for the same picture, field by
/// field (round 4, R4-5): the incoming record fills
/// only what the existing record leaves empty, and a
/// value already noted is never overwritten, so a
/// split-strength pair (the bulletin holds the credit
/// line, the sheet holds the license) ends up carrying
/// both. The children note merges as OR: a "children
/// are in this picture" from either side stands,
/// because the web page's hold-back is only as strong
/// as the strongest note.
///
/// No merge manufactures a permission, in any order
/// (round 5, F2, as a directional fence; round 6,
/// R6-1, as a property of the composed record). After
/// the fills, if the composed record notes children
/// and notes a permission, that permission must be
/// vouched for: a permission-satisfying field (the
/// credit line; a license that itself counts as a
/// noted permission, the church's own work, public
/// domain, CC0) stands only when the record it came
/// from itself notes children, or a children-bearing
/// record already noted a permission on its own
/// fields before this merge. Otherwise the field
/// reverts to the children-bearing record's value for
/// that field. The fill-only merge knows which side
/// supplied each field, so the check is on provenance,
/// not on which record happened to stand first, and
/// a fold of three or more records holds by
/// composition: every single merge leaves the
/// invariant standing. (The one step a fold defers
/// to its own end is the final judgment below —
/// round 8, R8-9, at
/// `fold_duplicate_asset_records`.)
///
/// A license and its credit travel together (round 6,
/// RT6-2): when a merge takes the incoming record's
/// license but the composed record ends up holding
/// that license with an empty credit line, although
/// the incoming record carried a credit line that did
/// not stand (refused above, or otherwise not
/// filled), the license does not fill either. No
/// merge leaves a record claiming a license whose
/// required attribution was dropped in the same merge.
///
/// One final judgment judges the composed record,
/// not the direction (round 7, R7-1, as the
/// togetherness rule; round 9, R9-1, as the only
/// judgment): after all the fills and reversions
/// above, if the composed record notes children and
/// its credit line is empty and it carries a
/// license, that license stands only if vouched —
/// some record in the composition held that same
/// license together with a children note, or
/// together with the credit that stands in the
/// composed record, or a children-bearing record in
/// the composition noted a permission under the
/// fence's bearer rule. This holds for every license
/// class, a reprint license included, and whichever
/// path composed the record: the license filled,
/// already stood, or would stand as a fallback. An
/// attribution license (CC BY, CC BY-SA) is never
/// vouched on an empty credit line. Unvouched, a
/// license falls to another license in the
/// composition that is vouched under this same
/// judgment — chosen order-independently, a license
/// carried by a children-bearing record first, then
/// by a fixed precedence (the church's own work,
/// public domain, CC0, a reprint license) — and to
/// Unknown where nothing vouches a license. A
/// children-bearing record whose own credit line
/// stands keeps its license. One part of the
/// judgment is a safety axis (round 11, R11-1):
/// when the composed record notes children and its
/// credit line is empty, a vouched license that
/// itself counts as a noted permission stands in
/// preference to a non-noting license, however well
/// vouched, so whether a permission stands beside
/// the children note is decided over the
/// composition the same way on both routes.
///
/// The assertion fields (asserted_by, asserted_at)
/// fill only when a substantive rights fill from the
/// same incoming record stands in the composed
/// record (round 5; round 7, R7-8): a credit line, or
/// a license that itself stands. An assertion vouches
/// for a rights fill and never rides in alone, on an
/// unrelated fill (a title, a source URL), or on a
/// fill refused above. The caller compares the result
/// with the existing record to know whether anything
/// was filled in.
pub fn merge_rights_records(existing: &RightsRecord, incoming: &RightsRecord) -> RightsRecord {
    merge_rights_records_inner(existing, incoming, false)
}

/// `merge_rights_records`, with the final judgment
/// optionally deferred (round 8, R8-9): a fold defers
/// that one step — the license fall for an empty
/// credit line — so a credit arriving later in the
/// same fold can stand beside the license it belongs
/// to, and judges it once, on the folded record, at
/// the fold's end (see `fold_duplicate_asset_records`).
/// Every other step — the fills, the fence reversions,
/// the incoming license-and-credit pairing, the
/// assertion rule — runs per merge exactly as in a
/// single merge.
fn merge_rights_records_inner(
    existing: &RightsRecord,
    incoming: &RightsRecord,
    defer_composed_rule: bool,
) -> RightsRecord {
    fn fill(existing: &str, incoming: &str) -> String {
        if existing.trim().is_empty() && !incoming.trim().is_empty() {
            incoming.to_string()
        } else {
            existing.to_string()
        }
    }
    // Provenance for the two permission-satisfying
    // fields: which side supplied the composed value.
    let credit_from_incoming =
        existing.credit_text.trim().is_empty() && !incoming.credit_text.trim().is_empty();
    let license_from_incoming =
        existing.license == LicenseKind::Unknown && incoming.license != LicenseKind::Unknown;
    let mut out = existing.clone();
    out.source = fill(&existing.source, &incoming.source);
    if existing.source_url.is_none() {
        out.source_url = incoming.source_url.clone();
    }
    if existing.license == LicenseKind::Unknown {
        out.license = incoming.license.clone();
    }
    out.title = fill(&existing.title, &incoming.title);
    out.creator = fill(&existing.creator, &incoming.creator);
    out.copyright_line = fill(&existing.copyright_line, &incoming.copyright_line);
    out.credit_text = fill(&existing.credit_text, &incoming.credit_text);
    out.licensor_ref = fill(&existing.licensor_ref, &incoming.licensor_ref);
    let children_present = match (existing.children_present, incoming.children_present) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), _) | (_, Some(false)) => Some(false),
        _ => None,
    };
    if children_present == Some(true) {
        // The record (or records) bearing the children
        // note, and whether any of them already noted a
        // permission on its own fields before this merge.
        let bearer = if existing.children_present == Some(true) {
            existing
        } else {
            incoming
        };
        let bearer_noted_permission = [existing, incoming]
            .iter()
            .any(|r| r.children_present == Some(true) && rights_permission_noted(r));
        if !out.credit_text.trim().is_empty() {
            let credit_source = if credit_from_incoming {
                incoming
            } else {
                existing
            };
            if credit_source.children_present != Some(true) && !bearer_noted_permission {
                let reverted_credit = bearer.credit_text.clone();
                out.credit_text = reverted_credit;
            }
        }
        if matches!(
            out.license,
            LicenseKind::ChurchOwned | LicenseKind::PublicDomain | LicenseKind::Cc0
        ) {
            let license_source = if license_from_incoming {
                incoming
            } else {
                existing
            };
            if license_source.children_present != Some(true) && !bearer_noted_permission {
                let reverted_license = bearer.license.clone();
                out.license = reverted_license;
            }
        }
    }
    // A license and its credit travel together: the
    // incoming license does not stand over an empty
    // credit line when the incoming record's own credit
    // line did not stand in this merge.
    if license_from_incoming
        && out.license == incoming.license
        && !incoming.credit_text.trim().is_empty()
        && out.credit_text.trim().is_empty()
    {
        let standing_license = existing.license.clone();
        out.license = standing_license;
    }
    // The final judgment on the composed record
    // (round 9, R9-1): ONE judgment, on every license
    // class, whatever path composed the record (the
    // license filled, already stood, or would stand
    // as a fallback). It subsumes the round-7
    // togetherness rule (an attribution license does
    // not stand beside a children note with an empty
    // credit line) and the round-8 uniform fallback
    // condition. The judgment itself lives at
    // `judged_composed_license`, shared with the
    // fold's end (where it is deferred to, so a credit
    // arriving later in the same fold can stand
    // beside the license it belongs to).
    if !defer_composed_rule {
        let sources: [&RightsRecord; 2] = [existing, incoming];
        let judged =
            judged_composed_license(&out.license, children_present, &out.credit_text, &sources);
        out.license = judged;
    }
    out.children_present = children_present;
    // The assertion stands only with a substantive
    // rights fill from the same incoming record that
    // itself stands in the composed record (round 7,
    // R7-8): its credit line, or its license.
    let credit_stands_from_incoming =
        credit_from_incoming && out.credit_text == incoming.credit_text;
    let license_stands_from_incoming = license_from_incoming && out.license == incoming.license;
    if credit_stands_from_incoming || license_stands_from_incoming {
        out.asserted_by = fill(&existing.asserted_by, &incoming.asserted_by);
        out.asserted_at = fill(&existing.asserted_at, &incoming.asserted_at);
    }
    out
}

/// Is a license vouched, under the final judgment
/// (round 9, R9-1)? A license in a composition that
/// notes children, with the credit line that stands
/// in the composed record, is vouched when some
/// record in the composition held that same license
/// together with a children note, or together with
/// the credit that stands in the composed record,
/// or when a children-bearing record in the
/// composition noted a permission on its own fields
/// (the fence's bearer rule: its credit line, or a
/// license that itself counts as a noted permission).
/// An attribution license (CC BY, CC BY-SA) is never
/// vouched on an empty credit line: its credit is
/// the license's own requirement, so with no credit
/// standing it falls whatever else vouches it.
fn license_vouched(
    license: &LicenseKind,
    standing_credit: &str,
    sources: &[&RightsRecord],
) -> bool {
    if matches!(license, LicenseKind::CcBy(_) | LicenseKind::CcBySa(_))
        && standing_credit.trim().is_empty()
    {
        return false;
    }
    let held_with_children = sources
        .iter()
        .any(|r| r.license == *license && r.children_present == Some(true));
    if held_with_children {
        return true;
    }
    let held_with_standing_credit = !standing_credit.trim().is_empty()
        && sources
            .iter()
            .any(|r| r.license == *license && r.credit_text == standing_credit);
    if held_with_standing_credit {
        return true;
    }
    sources
        .iter()
        .any(|r| r.children_present == Some(true) && rights_permission_noted(r))
}

/// The fixed precedence among fallback licenses
/// (round 9, R9-4), strongest claim first: the
/// church's own work, then public domain, then CC0,
/// then a reprint license (CCLI, One License), which
/// rests on an outside licensor and stands last.
fn fallback_rank(license: &LicenseKind) -> u8 {
    match license {
        LicenseKind::ChurchOwned => 0,
        LicenseKind::PublicDomain => 1,
        LicenseKind::Cc0 => 2,
        LicenseKind::LicensedReprint => 3,
        _ => 4,
    }
}

/// Pick the fallback license for a composed record
/// whose standing license falls (round 9, R9-1 and
/// R9-4): a license one of the composition's records
/// carried, vouched under the same judgment as a
/// standing license. Selection is order-independent:
/// a vouched license carried by a children-bearing
/// source record beats one vouched only through the
/// bearer rule, and among equals the fixed precedence
/// at `fallback_rank` decides, never the order the
/// records arrived in.
fn fallback_license(standing_credit: &str, sources: &[&RightsRecord]) -> Option<LicenseKind> {
    let mut best: Option<(u8, u8, LicenseKind)> = None;
    for source in sources {
        let candidate = &source.license;
        if matches!(
            candidate,
            LicenseKind::Unknown | LicenseKind::CcBy(_) | LicenseKind::CcBySa(_)
        ) {
            continue;
        }
        if best.as_ref().is_some_and(|(_, _, held)| held == candidate) {
            continue;
        }
        if !license_vouched(candidate, standing_credit, sources) {
            continue;
        }
        let carried_by_children_bearer = sources
            .iter()
            .any(|r| r.license == *candidate && r.children_present == Some(true));
        let tier: u8 = if carried_by_children_bearer { 0 } else { 1 };
        let rank = fallback_rank(candidate);
        let is_better = match &best {
            None => true,
            Some((held_tier, held_rank, _)) => (tier, rank) < (*held_tier, *held_rank),
        };
        if is_better {
            let chosen = candidate.clone();
            best = Some((tier, rank, chosen));
        }
    }
    best.map(|(_, _, license)| license)
}

/// The precedence among attribution licenses for
/// the credit-pairing tie-break (round 11, R11-9):
/// CC BY before CC BY-SA (the fallback precedence's
/// own spirit — the plainer claim first), then the
/// version string, so two records carrying the same
/// license kind at different versions still order
/// deterministically.
fn attribution_rank(license: &LicenseKind) -> (u8, String) {
    match license {
        LicenseKind::CcBy(version) => (0, version.clone()),
        LicenseKind::CcBySa(version) => (1, version.clone()),
        _ => (2, String::new()),
    }
}

/// The attribution license that travels with the
/// standing credit (round 10, R10-2): a license some
/// record in the composition held together with the
/// very credit line that stands in the composed
/// record. Returns that license when it outranks
/// the standing license under the pairing rule:
/// the standing license is not itself vouched in
/// this paired way (no record in the composition
/// held the standing license together with the
/// standing credit), or it is so vouched but
/// another attribution license vouched by the same
/// credit outranks it. Where several attribution
/// licenses travel with the one standing credit
/// (round 11, R11-9: two licenses carrying the SAME
/// credit text), the winner is chosen by the
/// standing precedence order (`attribution_rank`),
/// never by the order the records arrived in — the
/// fold used to take the first pairing in fold
/// order and a chain of single merges another, and
/// the choice between two vouched pairings was
/// arrival order, undisclosed. Where the standing
/// license is itself the precedence winner among
/// the credit-paired licenses, or no attribution
/// license is vouched by the standing credit, there
/// is nothing to answer with and the rest of the
/// judgment runs unchanged.
fn credit_paired_attribution_license(
    license: &LicenseKind,
    standing_credit: &str,
    sources: &[&RightsRecord],
) -> Option<LicenseKind> {
    if standing_credit.trim().is_empty() {
        return None;
    }
    let candidates: Vec<LicenseKind> = sources
        .iter()
        .filter(|r| {
            r.credit_text == standing_credit
                && matches!(r.license, LicenseKind::CcBy(_) | LicenseKind::CcBySa(_))
        })
        .map(|r| r.license.clone())
        .collect();
    credit_paired_from_candidates(license, &candidates)
}

/// The pairing step over an explicit candidate set:
/// the precedence winner among the attribution
/// licenses that travel with the standing credit,
/// returned when it is not the standing license
/// itself. Shared by the judgment over a merge's
/// own sources and by the fold's end, which passes
/// the pairings its merges actually saw (see
/// `fold_duplicate_asset_records`).
fn credit_paired_from_candidates(
    license: &LicenseKind,
    candidates: &[LicenseKind],
) -> Option<LicenseKind> {
    let mut best: Option<LicenseKind> = None;
    for candidate in candidates {
        let is_better = match &best {
            None => true,
            Some(held) => attribution_rank(candidate) < attribution_rank(held),
        };
        if is_better {
            best = Some(candidate.clone());
        }
    }
    match best {
        Some(winner) if winner != *license => Some(winner),
        _ => None,
    }
}

/// The best vouched permission-noting license in
/// a composition (round 11, R11-1): a license that
/// itself counts as a noted permission (the church's
/// own work, public domain, CC0 — the licenses in
/// `rights_permission_noted`), vouched under the
/// same judgment as a standing license (`license_vouched`
/// with the empty credit line of the branch that
/// calls this). Selection mirrors `fallback_license`:
/// a license carried by a children-bearing source
/// record first, then the fixed precedence at
/// `fallback_rank`, never the order the records
/// arrived in.
fn permission_fallback_license(sources: &[&RightsRecord]) -> Option<LicenseKind> {
    let mut best: Option<(u8, u8, LicenseKind)> = None;
    for source in sources {
        let candidate = &source.license;
        if !matches!(
            candidate,
            LicenseKind::ChurchOwned | LicenseKind::PublicDomain | LicenseKind::Cc0
        ) {
            continue;
        }
        if best.as_ref().is_some_and(|(_, _, held)| held == candidate) {
            continue;
        }
        if !license_vouched(candidate, "", sources) {
            continue;
        }
        let carried_by_children_bearer = sources
            .iter()
            .any(|r| r.license == *candidate && r.children_present == Some(true));
        let tier: u8 = if carried_by_children_bearer { 0 } else { 1 };
        let rank = fallback_rank(candidate);
        let is_better = match &best {
            None => true,
            Some((held_tier, held_rank, _)) => (tier, rank) < (*held_tier, *held_rank),
        };
        if is_better {
            let chosen = candidate.clone();
            best = Some((tier, rank, chosen));
        }
    }
    best.map(|(_, _, license)| license)
}

/// The ONE final judgment on a composed record
/// (round 9, R9-1): if the composed record notes
/// children and its credit line is empty and it
/// carries a license, that license stands only if
/// vouched (see `license_vouched`). Unvouched, it
/// falls — to another license in the composition
/// that is vouched under this same judgment (see
/// `fallback_license`), else Unknown. Every other
/// composed record (no children note, a credit line
/// standing, or no license) is left exactly as it
/// stands: the publish fence above has already
/// judged the permission-satisfying fields on their
/// own provenance, and a standing credit vouches
/// through the bearer rule by construction.
///
/// One step runs before all of that (round 10,
/// R10-2): the license that travels with the credit
/// that stands wins, on the fold route and on a
/// single merge alike. The two routes used to pick
/// different licenses for the same records — the
/// fold kept the first license standing, a chain of
/// single merges ended with the credit record's
/// license, because each pairwise judgment dropped
/// or replaced the standing license before the
/// credit arrived. Now, when the standing license
/// is not the one vouched by the standing credit
/// but another attribution license in the
/// composition is (see
/// `credit_paired_attribution_license`), the
/// credit-paired license stands.
///
/// One more step runs inside the empty-credit
/// branch (round 11, R11-1): the permission axis is
/// a safety axis, route-invariant like the children
/// note itself. When children stand and the credit
/// line is empty, whether a permission stands
/// beside the note is decided over the sources this
/// judgment sees, by one rule on both routes: a
/// vouched permission-noting license in the
/// composition (see `permission_fallback_license`)
/// stands in preference to any non-noting license,
/// however well vouched the non-noting license is.
/// The routes used to disagree here in both
/// directions — the fold judged the whole
/// composition at its end, a chain of single merges
/// judged pair by pair, and a permission-noting
/// claim (a CC0 carried by a children-bearing
/// record) won on one route and lost to a vouched
/// reprint license on the other, so the same
/// children's picture rode the web page or was held
/// back by the accident of how its records arrived.
/// This step does NOT give a single merge the
/// fold's full end-state judgment: it runs on the
/// sources each route's own judgment already sees,
/// and because it runs at every pairwise judgment,
/// a permission-noting claim that stands in any
/// prefix of a single-merge chain is carried into
/// the chain's later judgments by the composed
/// record itself.
fn judged_composed_license(
    license: &LicenseKind,
    children_present: Option<bool>,
    credit_text: &str,
    sources: &[&RightsRecord],
) -> LicenseKind {
    judged_composed_license_inner(license, children_present, credit_text, sources, None)
}

/// `judged_composed_license`, with the credit-paired
/// candidate set optionally supplied by the caller
/// instead of read off `sources`. Only the fold's
/// end supplies it (round 11, R11-9): the fold judges
/// pairings its merges actually saw, tracked by
/// `fold_duplicate_asset_records`, because a pairing
/// whose credit the fence reverted at its own merge
/// is no longer visible to a chain of single merges
/// either — counting it at fold end would pick a
/// license the other route cannot reach.
fn judged_composed_license_inner(
    license: &LicenseKind,
    children_present: Option<bool>,
    credit_text: &str,
    sources: &[&RightsRecord],
    paired_candidates: Option<&[LicenseKind]>,
) -> LicenseKind {
    let paired = match paired_candidates {
        Some(candidates) if !credit_text.trim().is_empty() => {
            credit_paired_from_candidates(license, candidates)
        }
        _ => credit_paired_attribution_license(license, credit_text, sources),
    };
    if let Some(paired) = paired {
        return paired;
    }
    if children_present != Some(true) {
        return license.clone();
    }
    if !credit_text.trim().is_empty() {
        return license.clone();
    }
    // The permission axis (round 11, R11-1), stated
    // above: a standing permission-noting license
    // that is itself vouched stands (standing-wins
    // among noting claims); otherwise a vouched
    // permission-noting license anywhere in the
    // composition stands in preference to a
    // non-noting one.
    let license_noted = matches!(
        license,
        LicenseKind::ChurchOwned | LicenseKind::PublicDomain | LicenseKind::Cc0
    );
    if license_noted && license_vouched(license, credit_text, sources) {
        return license.clone();
    }
    if let Some(permission) = permission_fallback_license(sources) {
        return permission;
    }
    if *license == LicenseKind::Unknown {
        return license.clone();
    }
    if license_vouched(license, credit_text, sources) {
        return license.clone();
    }
    fallback_license(credit_text, sources).unwrap_or(LicenseKind::Unknown)
}

/// Re-judge a folded record's assertion against the
/// FINAL folded record (round 9, R9-5), by the R7-8
/// rule a single merge applies: the assertion stands
/// only if a substantive rights fill from its own
/// record stands there — that record's credit line,
/// or that record's license. In a fold the per-merge
/// assertion rule runs while a license still stands
/// under deferral; when the fold-end judgment drops
/// that license, the assertion that rode in on it
/// does not stay behind.
fn rejudge_folded_assertion(folded: &mut RightsRecord, sources: &[RightsRecord]) {
    if folded.asserted_by.trim().is_empty() && folded.asserted_at.trim().is_empty() {
        return;
    }
    let supplier_stands = sources.iter().any(|source| {
        let is_supplier = (!folded.asserted_by.trim().is_empty()
            && source.asserted_by == folded.asserted_by)
            || (!folded.asserted_at.trim().is_empty() && source.asserted_at == folded.asserted_at);
        if !is_supplier {
            return false;
        }
        let credit_stands =
            !source.credit_text.trim().is_empty() && folded.credit_text == source.credit_text;
        let license_stands =
            source.license != LicenseKind::Unknown && folded.license == source.license;
        credit_stands || license_stands
    });
    if !supplier_stands {
        folded.asserted_by = String::new();
        folded.asserted_at = String::new();
    }
}

/// A credit line vouched in a fold (round 11,
/// R11-5), under the fence's own rule read with the
/// fold's arrival timing: the credit's own record
/// notes the children, or a children-bearing record
/// standing at or before it in fold order noted a
/// permission on its own fields. The first such
/// credit in fold order is the one — the fill order
/// the merges themselves follow. A credit vouched
/// only by a bearer that arrives AFTER it is not
/// restored: a chain of single merges drops that
/// credit at its arrival, before the bearer is
/// seen, and the fold keeps the same answer (the
/// credit class the round-8 deferral already
/// leaves to arrival timing), while a credit whose
/// bearer stands before it — the R11-5 shape — is
/// the one the fold used to lose.
fn vouched_fold_credit(sources: &[RightsRecord]) -> Option<String> {
    let mut bearer_noted_permission = false;
    for source in sources {
        if !source.credit_text.trim().is_empty()
            && (source.children_present == Some(true) || bearer_noted_permission)
        {
            return Some(source.credit_text.clone());
        }
        if source.children_present == Some(true) && rights_permission_noted(source) {
            bearer_noted_permission = true;
        }
    }
    None
}

/// The attribution licenses that travel with a
/// restored credit (round 11, R11-9, companion to
/// `vouched_fold_credit`): the pairings of the
/// credit's prefix-vouched carriers — the records
/// carrying that credit that pass the same arrival
/// test the restored credit itself passed (their
/// own record notes the children, or a
/// children-bearing permission-noting record stands
/// at or before them). A carrier the bearer had not
/// yet vouched when it arrived contributes no
/// pairing: a chain of single merges dropped that
/// carrier's license at its arrival, before the
/// credit was restored, and cannot reach it again.
fn restored_credit_pairings(sources: &[RightsRecord], credit: &str) -> Vec<LicenseKind> {
    let mut bearer_noted_permission = false;
    let mut pairings: Vec<LicenseKind> = Vec::new();
    for source in sources {
        let prefix_vouched = source.children_present == Some(true) || bearer_noted_permission;
        if prefix_vouched
            && source.credit_text == credit
            && matches!(
                source.license,
                LicenseKind::CcBy(_) | LicenseKind::CcBySa(_)
            )
            && !pairings.contains(&source.license)
        {
            pairings.push(source.license.clone());
        }
        if source.children_present == Some(true) && rights_permission_noted(source) {
            bearer_noted_permission = true;
        }
    }
    pairings
}

/// Judge the final judgment once, on a folded
/// record (round 8, R8-9; round 9, R9-1): the same
/// `judged_composed_license` a single merge applies,
/// run over the fold's own source records, and then
/// the assertion re-judged against what finally
/// stands (round 9, R9-5).
///
/// Before that judgment, one fold-only repair
/// (round 11, R11-5): a fold defers the license
/// judgment to its end, so during the fold a
/// children-bearing record's permission-noting
/// license may never stand in the composed record —
/// the attribution license it will eventually fall
/// to a fallback for is still standing in its
/// place — and the per-merge fence, seeing no
/// permission noted on the pair before it, reverts
/// a late-arriving credit line that the composition
/// as a whole vouches for. The fold then ended with
/// the credit silently gone and the fallback
/// license (which carries no credit) standing in
/// its place, while a chain of single merges —
/// whose first pairwise judgment had already
/// dropped the attribution license to the
/// permission-noting fallback — let the same credit
/// stand and paired its license beside it. So: when
/// the folded record notes children and its credit
/// line came out empty, a credit vouched under the
/// fence's rule with the fold's arrival timing (see
/// `vouched_fold_credit`) stands in the folded
/// record before the license is judged, and the
/// judgment's credit-pairing step then stands the
/// license that traveled with that credit beside
/// it. A fallback license never orphans the
/// composition's only vouched credit. No credit is
/// manufactured by this: the restored line is a
/// source record's own line, vouched under the
/// fence's rule, and a credit no source vouches
/// for stays reverted.
fn judge_folded_composed_record(
    folded: &mut RightsRecord,
    sources: &[RightsRecord],
    paired_candidates: &[LicenseKind],
) {
    let mut candidates: Vec<LicenseKind> = paired_candidates.to_vec();
    if folded.children_present == Some(true) && folded.credit_text.trim().is_empty() {
        if let Some(credit) = vouched_fold_credit(sources) {
            folded.credit_text = credit;
            // A restored credit stood at no merge, so
            // its pairings were never tracked; its
            // prefix-vouched carriers supply them (see
            // `restored_credit_pairings`), as a single
            // merge's own judgment would see on the
            // merges that let the credit stand.
            candidates = restored_credit_pairings(sources, &folded.credit_text);
        }
    }
    let source_refs: Vec<&RightsRecord> = sources.iter().collect();
    let judged = judged_composed_license_inner(
        &folded.license,
        folded.children_present,
        &folded.credit_text,
        &source_refs,
        Some(&candidates),
    );
    folded.license = judged;
    rejudge_folded_assertion(folded, sources);
}

/// Fold records that share a hash into one record
/// apiece (round 5, RT5-1): a file may carry two
/// records for the same picture bytes, and every
/// reader looks a picture up by its hash and takes
/// the record it finds, so a weak duplicate standing
/// first silently shadowed the fuller one: a children
/// note disarmed, a license dropped. The loaders fold
/// on load, before any reader sees the records.
/// Duplicates merge field by field with
/// `merge_rights_records`; the first record's kind
/// and media type stand (the same bytes carry the
/// same kind). After the fold, one hash has exactly
/// one record, here and in every copy saved onward.
/// The merges run with the final judgment deferred,
/// and that judgment is run once, on the folded
/// record, at the end (round 8, R8-9; round 9, R9-1),
/// with the assertion then re-judged against what
/// finally stands (round 9, R9-5).
pub fn fold_duplicate_asset_records(assets: &mut Vec<Asset>) {
    let mut folded: Vec<Asset> = Vec::with_capacity(assets.len());
    // The records each folded record was composed
    // from, in fold order: the fold's final judging
    // (below) reads them for its fallback.
    let mut folded_sources: Vec<Vec<RightsRecord>> = Vec::with_capacity(assets.len());
    // The credit pairings each fold's merges actually
    // saw, in fold order (round 11, R11-9): after a
    // merge, the attribution licenses the merge's
    // source records held together with the credit
    // line the merge left standing — the incoming
    // record at every merge, and the standing record
    // too at the first merge, where it is still a
    // source record itself. (At later merges the
    // standing side is a composed record: its license
    // and a same-text credit filled from different
    // sources can sit together in it without any
    // source holding that pairing, and no single
    // merge counts such an accidental pairing — its
    // genuine pairing, if it has one, was counted at
    // the merge that formed it.) The fold's final
    // judgment chooses among these, not among every
    // pairing in the source set — a pairing whose
    // credit the fence reverted at its own merge is
    // invisible to a chain of single merges from
    // that merge on, and the routes must choose the
    // shared credit's license from the same
    // candidates.
    let mut folded_paired: Vec<Vec<LicenseKind>> = Vec::with_capacity(assets.len());
    // Pairings vouched on arrival, with the credit
    // text they travel with (round 11, R11-9): a
    // pairing whose credit is prefix-vouched at its
    // merge (see `vouched_fold_credit`) counts even
    // when the fold's deferred fence reverts that
    // credit pair-locally — the same blindness the
    // R11-5 restoration repairs — because a chain of
    // single merges, whose composed record already
    // carries the bearer's permission, lets the
    // credit stand at that merge and sees the
    // pairing. These are filtered to the credit that
    // finally stands, at the fold's end.
    let mut folded_vouched_pairs: Vec<Vec<(LicenseKind, String)>> =
        Vec::with_capacity(assets.len());
    // Whether a children-bearing permission-noting
    // record stands among the sources folded so far,
    // per folded record: the prefix the arrival test
    // reads.
    let mut folded_bearer_seen: Vec<bool> = Vec::with_capacity(assets.len());
    for asset in assets.drain(..) {
        match folded.iter().position(|a| a.sha256 == asset.sha256) {
            Some(pos) => {
                let first_merge = folded_sources[pos].len() == 1;
                folded_sources[pos].push(asset.rights.clone());
                let before = folded[pos].rights.clone();
                let incoming = asset.rights.clone();
                // The incoming record's pairing, vouched
                // on arrival: its own record notes the
                // children, or the fold's prefix already
                // holds a noting bearer.
                let arrival_vouched =
                    incoming.children_present == Some(true) || folded_bearer_seen[pos];
                if arrival_vouched
                    && !incoming.credit_text.trim().is_empty()
                    && matches!(
                        incoming.license,
                        LicenseKind::CcBy(_) | LicenseKind::CcBySa(_)
                    )
                {
                    let pair = (incoming.license.clone(), incoming.credit_text.clone());
                    if !folded_vouched_pairs[pos].contains(&pair) {
                        folded_vouched_pairs[pos].push(pair);
                    }
                }
                if incoming.children_present == Some(true) && rights_permission_noted(&incoming) {
                    folded_bearer_seen[pos] = true;
                }
                let merged = merge_rights_records_inner(&before, &incoming, true);
                if merged.credit_text.trim().is_empty() || merged.credit_text != before.credit_text
                {
                    // The credit these pairings traveled
                    // with no longer stands: a reverted
                    // credit takes its pairings with it,
                    // on this route as on a chain of
                    // single merges, whose composed
                    // record holds neither after the
                    // reversion.
                    folded_paired[pos].clear();
                }
                if !merged.credit_text.trim().is_empty() {
                    let sides: &[&RightsRecord] = if first_merge {
                        &[&before, &incoming]
                    } else {
                        &[&incoming]
                    };
                    for side in sides {
                        if side.credit_text == merged.credit_text
                            && matches!(side.license, LicenseKind::CcBy(_) | LicenseKind::CcBySa(_))
                            && !folded_paired[pos].contains(&side.license)
                        {
                            let seen = side.license.clone();
                            folded_paired[pos].push(seen);
                        }
                    }
                }
                folded[pos].rights = merged;
            }
            None => {
                let bearer = asset.rights.children_present == Some(true)
                    && rights_permission_noted(&asset.rights);
                folded_sources.push(vec![asset.rights.clone()]);
                folded_paired.push(Vec::new());
                folded_vouched_pairs.push(Vec::new());
                folded_bearer_seen.push(bearer);
                folded.push(asset);
            }
        }
    }
    // The final judgment is judged once, here, on
    // each folded record that a merge actually
    // composed (round 8, R8-9; round 9, R9-1): a
    // record no duplicate folded into is left exactly
    // as it stood, as before.
    for (((asset, sources), paired), vouched_pairs) in folded
        .iter_mut()
        .zip(folded_sources.iter())
        .zip(folded_paired.iter())
        .zip(folded_vouched_pairs.iter())
    {
        if sources.len() > 1 {
            // The candidates the fold's end chooses
            // among: the pairings its merges saw
            // standing, plus the arrival-vouched
            // pairings that travel with the credit
            // that finally stands.
            let mut candidates: Vec<LicenseKind> = paired.clone();
            for (license, credit) in vouched_pairs {
                if *credit == asset.rights.credit_text && !candidates.contains(license) {
                    candidates.push(license.clone());
                }
            }
            judge_folded_composed_record(&mut asset.rights, sources, &candidates);
        }
    }
    *assets = folded;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sha256_hex, ChurchProfile, Document, SheetSpec, Size, FORMAT_VERSION};
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
    fn assets_store_bytes_by_hash_with_rights() {
        let mut doc = bare_doc();
        let rights = RightsRecord {
            source: "the church camera".into(),
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let h1 = doc.add_asset(b"PNGDATA", AssetKind::Picture, "image/png", rights);
        assert_eq!(h1, sha256_hex(b"PNGDATA"));
        assert_eq!(doc.asset_data(&h1), Some(b"PNGDATA".as_slice()));
        assert_eq!(doc.assets.len(), 1);
        assert_eq!(doc.assets[0].kind, AssetKind::Picture);
        // The same bytes again are the same asset.
        let h2 = doc.add_asset(
            b"PNGDATA",
            AssetKind::Picture,
            "image/png",
            RightsRecord::default(),
        );
        assert_eq!(h1, h2);
        assert_eq!(doc.assets.len(), 1);
        // Rights and license kinds survive JSON (they ride in
        // document.json; the bytes never do).
        let json = serde_json::to_string(&doc).expect("json");
        assert!(!json.contains("PNGDATA"), "bytes stay out of the JSON");
        let back: Document = serde_json::from_str(&json).expect("parse");
        assert_eq!(back.assets[0].rights.license, LicenseKind::ChurchOwned);
        assert_eq!(back.assets[0].rights.source, "the church camera");
        assert!(back.asset_bytes.is_empty());
    }

    #[test]
    fn r5_add_asset_merges_the_rights_of_bytes_already_held() {
        // Round 5 (RT5-2): the same bytes arrive twice,
        // strong in different halves. The second record
        // used to be thrown away whole; now it fills
        // what the first leaves empty, and the first
        // record's noted values stand.
        let mut doc = bare_doc();
        let h1 = doc.add_asset(
            b"PNGDATA",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                source: "the church camera".into(),
                ..Default::default()
            },
        );
        let h2 = doc.add_asset(
            b"PNGDATA",
            AssetKind::Picture,
            "image/png",
            RightsRecord {
                title: "Harvest supper".into(),
                license: LicenseKind::CcBy("4.0".into()),
                ..Default::default()
            },
        );
        assert_eq!(h1, h2);
        assert_eq!(doc.assets.len(), 1);
        let rights = &doc.assets[0].rights;
        assert_eq!(rights.source, "the church camera");
        assert_eq!(rights.title, "Harvest supper");
        assert_eq!(rights.license, LicenseKind::CcBy("4.0".into()));
    }

    #[test]
    fn r5_merge_never_manufactures_a_web_permission() {
        // Round 5 (F2): the bulletin record notes
        // children in the picture and no permission;
        // the sheet record holds a credit line. The
        // merge must not complete the pair into a
        // noted permission.
        let bulletin = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let sheet = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&bulletin, &sheet);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.license, LicenseKind::Unknown);
        // A license that itself counts as a noted
        // permission is fenced the same way.
        let sheet_cc0 = RightsRecord {
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        assert_eq!(
            merge_rights_records(&bulletin, &sheet_cc0).license,
            LicenseKind::Unknown
        );
        // Fills that say nothing about permission
        // still land in a fenced record.
        // FLAGGED, round 7 (R7-1): this pin used to
        // assert the CC BY license standing here. The
        // composed record notes children and holds no
        // credit line at all, so the standing license
        // claimed an attribution nothing in the record
        // carries — the composed state RT7-1 graded
        // BLOCKER. Under the composed togetherness
        // rule the title still lands and the license
        // now falls to Unknown; the round-7 mirror
        // test pins the same state from the other
        // direction.
        let sheet_fuller = RightsRecord {
            title: "Harvest supper".into(),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let merged = merge_rights_records(&bulletin, &sheet_fuller);
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.license, LicenseKind::Unknown);
        // With no children note, nothing is fenced:
        // the ordinary fill (round 4, R4-5) carries
        // the credit line.
        let plain = RightsRecord::default();
        let merged = merge_rights_records(&plain, &sheet);
        assert_eq!(merged.credit_text, "Photo by Lee");
        // And a record whose permission is already
        // noted is not fenced at all.
        let noted = RightsRecord {
            children_present: Some(true),
            credit_text: "Photo by Lee, used with parent permission.".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&noted, &sheet_cc0);
        assert_eq!(merged.license, LicenseKind::Cc0);
    }

    #[test]
    fn r5_merge_assertion_fields_ride_only_with_a_real_fill() {
        // Round 5: asserted_by / asserted_at vouch for
        // a fill. An incoming record carrying only an
        // assertion changes nothing at all.
        let existing = RightsRecord {
            source: "the church camera".into(),
            ..Default::default()
        };
        let assertion_only = RightsRecord {
            asserted_by: "Sam".into(),
            asserted_at: "2026-10-16".into(),
            ..Default::default()
        };
        assert_eq!(merge_rights_records(&existing, &assertion_only), existing);
        // With a substantive fill from the same record,
        // the assertion rides along with it.
        // FLAGGED, round 7 (R7-8): the fill here used
        // to be a title. "Substantive" now means
        // rights-bearing — a credit line or a license
        // that itself stands — so the fill is a credit
        // line; a title alone no longer carries an
        // assertion, and the round-7 assertion test
        // pins that shape by name.
        let with_fill = RightsRecord {
            credit_text: "Photo by Jane Doe".into(),
            asserted_by: "Sam".into(),
            asserted_at: "2026-10-16".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &with_fill);
        assert_eq!(merged.credit_text, "Photo by Jane Doe");
        assert_eq!(merged.asserted_by, "Sam");
        assert_eq!(merged.asserted_at, "2026-10-16");
    }

    #[test]
    fn r6_merge_never_manufactures_a_permission_in_either_order() {
        // Round 6 (R6-1 / RT6-1): the round-5 fence faced
        // one way. A credit-bearing record standing first
        // and a children-noted record second composed a
        // permission no single record ever claimed. The
        // property is on the composed record now, so both
        // orders end the same way.
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        for merged in [
            merge_rights_records(&credit, &children),
            merge_rights_records(&children, &credit),
        ] {
            assert_eq!(merged.children_present, Some(true));
            assert_eq!(merged.credit_text, "");
            assert!(!rights_permission_noted(&merged));
        }
        // A permission license standing first is fenced
        // the same way, in either order.
        let cc0 = RightsRecord {
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let church_owned = RightsRecord {
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        for merged in [
            merge_rights_records(&cc0, &children),
            merge_rights_records(&children, &cc0),
            merge_rights_records(&church_owned, &children),
            merge_rights_records(&children, &church_owned),
        ] {
            assert_eq!(merged.children_present, Some(true));
            assert_eq!(merged.license, LicenseKind::Unknown);
            assert!(!rights_permission_noted(&merged));
        }
        // A credit arriving on a record that itself notes
        // children may stand: that record carried both.
        let children_with_credit = RightsRecord {
            credit_text: "Photo by Lee, used with parent permission.".into(),
            children_present: Some(true),
            ..Default::default()
        };
        let merged = merge_rights_records(&children, &children_with_credit);
        assert_eq!(
            merged.credit_text,
            "Photo by Lee, used with parent permission."
        );
        assert!(rights_permission_noted(&merged));
        // A children record that noted its own permission
        // keeps it and accepts other fills.
        let fuller = RightsRecord {
            title: "Harvest supper".into(),
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let merged = merge_rights_records(&children_with_credit, &fuller);
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.license, LicenseKind::Cc0);
        assert_eq!(
            merged.credit_text,
            "Photo by Lee, used with parent permission."
        );
        // With no children note anywhere, the ordinary
        // split-strength merge (round 4, R4-5) still
        // carries both halves.
        let license_only = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let merged = merge_rights_records(&credit, &license_only);
        assert_eq!(merged.credit_text, "Photo by Lee");
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.children_present, None);
    }

    #[test]
    fn r6_three_record_fold_never_composes_a_permission() {
        // Round 6 (R6-1): a fold of three records (a
        // credit record, a children record, a neutral
        // record) in mixed orders never composes a
        // permission, and the neutral fills still land.
        let hash = sha256_hex(b"R6FOLD");
        let credit = RightsRecord {
            credit_text: "Photo by Lee".into(),
            ..Default::default()
        };
        let children = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let neutral = RightsRecord {
            title: "Harvest supper".into(),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        for records in [
            vec![credit.clone(), children.clone(), neutral.clone()],
            vec![credit.clone(), neutral.clone(), children.clone()],
            vec![children.clone(), neutral.clone(), credit.clone()],
            vec![neutral.clone(), credit.clone(), children.clone()],
        ] {
            let mut assets: Vec<Asset> = records.into_iter().map(asset).collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            let rights = &assets[0].rights;
            assert_eq!(rights.children_present, Some(true));
            assert_eq!(rights.title, "Harvest supper");
            assert!(!rights_permission_noted(rights), "{rights:?}");
        }
    }

    #[test]
    fn r6_merge_license_and_its_credit_travel_together() {
        // Round 6 (RT6-2): into a children-noted record,
        // the old fence blocked the incoming credit line
        // but let its CC BY license through, claiming a
        // license whose required attribution was dropped.
        // The pair travels together or not at all, while
        // fills that say nothing about the pair still land.
        let existing = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let incoming = RightsRecord {
            title: "Church picnic".into(),
            creator: "Alice".into(),
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &incoming);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.license, LicenseKind::Unknown);
        assert_eq!(merged.title, "Church picnic");
        assert_eq!(merged.creator, "Alice");
        // The refused pair counts as no fill at all: with
        // nothing else in the incoming record, the merge
        // changes nothing and no assertion rides in.
        let pair_only = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            asserted_by: "Sam".into(),
            asserted_at: "2026-10-16".into(),
            ..Default::default()
        };
        assert_eq!(merge_rights_records(&existing, &pair_only), existing);
        // When a credit line already stands on the
        // existing record, the license may fill: a credit
        // stands, which is not the harm above.
        let credited = RightsRecord {
            credit_text: "Photo by Alice, used with permission.".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&credited, &incoming);
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.credit_text, "Photo by Alice, used with permission.");
    }

    #[test]
    fn r6_add_asset_never_manufactures_a_permission() {
        // Round 6 (R6-1): add_asset composes through the
        // same merge. Bytes first held with a credit line
        // and later given a children note (and the other
        // order) never end up noting a permission.
        for first_credit in [true, false] {
            let mut doc = bare_doc();
            let credit = RightsRecord {
                credit_text: "Photo by Lee".into(),
                ..Default::default()
            };
            let children = RightsRecord {
                children_present: Some(true),
                ..Default::default()
            };
            let (first, second) = if first_credit {
                (credit, children)
            } else {
                (children, credit)
            };
            doc.add_asset(b"R6PIC", AssetKind::Picture, "image/png", first);
            doc.add_asset(b"R6PIC", AssetKind::Picture, "image/png", second);
            assert_eq!(doc.assets.len(), 1);
            let rights = &doc.assets[0].rights;
            assert_eq!(rights.children_present, Some(true));
            assert!(!rights_permission_noted(rights), "{rights:?}");
        }
    }

    #[test]
    fn r7_a_standing_license_loses_its_credit_and_falls() {
        // Round 7 (R7-1 / RT7-1, the RT7-1 probe
        // verbatim): the license already stands on the
        // children-bearing record and the only credit
        // arrives from a second record, where the fence
        // refuses it. The composed record used to keep
        // the license with an empty credit line — the
        // state RT6-2 was graded BLOCKER for, composed
        // in the mirror direction. The togetherness
        // rule judges the composed record now: the
        // license falls, the children note stands, for
        // both attribution licenses.
        let credit = RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        for license in [
            LicenseKind::CcBy("4.0".into()),
            LicenseKind::CcBySa("4.0".into()),
        ] {
            let existing = RightsRecord {
                license: license.clone(),
                children_present: Some(true),
                ..Default::default()
            };
            let merged = merge_rights_records(&existing, &credit);
            assert_eq!(merged.children_present, Some(true), "{license:?}");
            assert_eq!(merged.credit_text, "", "{license:?}");
            assert_eq!(merged.license, LicenseKind::Unknown, "{license:?}");
        }
        // Control (the R5b shape): a children-bearing
        // record whose OWN credit stands keeps its
        // license when an unrelated record merges in.
        let own_credit = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            children_present: Some(true),
            ..Default::default()
        };
        let unrelated = RightsRecord {
            title: "Picnic".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&own_credit, &unrelated);
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.credit_text, "Photo by Alice, CC BY 4.0");
        assert_eq!(merged.title, "Picnic");
        // Licenses that require no attribution are
        // untouched by the rule: a children-bearing
        // record's CC0 stands with an empty credit
        // line after an unrelated merge. (A CC0 record
        // is itself a noted permission, so a credit
        // arriving beside it stands under the round-6
        // bearer rule and cannot show this.)
        let cc0_bearer = RightsRecord {
            license: LicenseKind::Cc0,
            children_present: Some(true),
            ..Default::default()
        };
        let merged = merge_rights_records(&cc0_bearer, &unrelated);
        assert_eq!(merged.license, LicenseKind::Cc0);
        assert_eq!(merged.credit_text, "");
    }

    #[test]
    fn r7_a_dropped_license_leaves_a_vouched_license_standing() {
        // Round 7 (R7-1): the dropped license reverts
        // to what the merge holds without it. Here the
        // other record is itself children-bearing and
        // vouches a CC0 license, so when the CC BY
        // falls that license is what the merge holds.
        let ccby = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let cc0 = RightsRecord {
            license: LicenseKind::Cc0,
            children_present: Some(true),
            ..Default::default()
        };
        for merged in [
            merge_rights_records(&ccby, &cc0),
            merge_rights_records(&cc0, &ccby),
        ] {
            assert_eq!(merged.children_present, Some(true));
            assert_eq!(merged.credit_text, "");
            assert_eq!(merged.license, LicenseKind::Cc0);
        }
    }

    #[test]
    fn r7_three_record_fold_drops_the_orphaned_license() {
        // Round 7 (R7-1): the three-record fold
        // [children+CC BY, title, credit] through the
        // load fold's own funnel. The credit is refused
        // at the last merge, so the license that stood
        // from the first record falls with it; the
        // neutral title fill still lands.
        let hash = sha256_hex(b"R7FOLD");
        let children_ccby = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let titled = RightsRecord {
            title: "Picnic".into(),
            ..Default::default()
        };
        let credit = RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let mut assets: Vec<Asset> = vec![children_ccby, titled, credit]
            .into_iter()
            .map(asset)
            .collect();
        fold_duplicate_asset_records(&mut assets);
        assert_eq!(assets.len(), 1);
        let rights = &assets[0].rights;
        assert_eq!(rights.children_present, Some(true));
        assert_eq!(rights.license, LicenseKind::Unknown);
        assert_eq!(rights.credit_text, "");
        assert_eq!(rights.title, "Picnic");
    }

    #[test]
    fn r8_unvouched_licensed_reprint_fallback_falls() {
        // Round 8 (R8-3 / R8-R2, the review probe
        // verbatim): the existing record notes
        // children and a CC BY license with no credit;
        // the incoming record carries LicensedReprint
        // only — no children note, no credit, no
        // permission of any kind. The composed rule
        // drops the CC BY, and the fallback used to
        // install the LicensedReprint unconditionally,
        // displacing the standing license with a claim
        // no record vouched. The fallback arms are
        // uniform now: an unvouched LicensedReprint
        // falls to Unknown like any other.
        let existing = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: String::new(),
            ..Default::default()
        };
        let incoming = RightsRecord {
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &incoming);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.license, LicenseKind::Unknown);
        // The same pair through a fold composes the
        // same way.
        let hash = sha256_hex(b"R8REPRINT");
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        let mut assets: Vec<Asset> = vec![existing.clone(), incoming]
            .into_iter()
            .map(asset)
            .collect();
        fold_duplicate_asset_records(&mut assets);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].rights.children_present, Some(true));
        assert_eq!(assets[0].rights.license, LicenseKind::Unknown);
        // Control: a LicensedReprint whose own source
        // record notes children IS vouched, and stands
        // where the dropped CC BY stood — the uniform
        // condition, not a blanket fall.
        let vouched = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &vouched);
        assert_eq!(merged.license, LicenseKind::LicensedReprint);
    }

    #[test]
    fn r8_fallback_controls_from_the_cleared_shapes() {
        // Round 8 (R8-3): the review lane's cleared
        // controls, pinned where they were not already
        // pinned. (The vouched-CC0 fallback and the
        // own-credit control are already pinned by
        // `r7_a_dropped_license_leaves_a_vouched_license_standing`
        // and `r7_a_standing_license_loses_its_credit_and_falls`.)
        let existing = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        // A ChurchOwned license from a source that
        // notes no children falls to Unknown.
        let church_owned = RightsRecord {
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &church_owned);
        assert_eq!(merged.license, LicenseKind::Unknown);
        // A CC BY-SA "other" license falls too:
        // attribution licenses never stand as the
        // fallback.
        let cc_by_sa = RightsRecord {
            license: LicenseKind::CcBySa("4.0".into()),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &cc_by_sa);
        assert_eq!(merged.license, LicenseKind::Unknown);
        // The mirror direction, exactly as cleared: a
        // standing LicensedReprint, an incoming CC BY
        // whose credit the fence refuses — the
        // standing license stays and the credit is
        // refused.
        let credit_text = "Photo by Alice, CC BY 4.0".to_string();
        let standing = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let incoming = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let merged = merge_rights_records(&standing, &incoming);
        assert_eq!(merged.children_present, Some(true));
        assert_eq!(merged.license, LicenseKind::LicensedReprint);
        assert_eq!(merged.credit_text, "");
    }

    #[test]
    fn r8_fold_judges_the_composed_rule_once_at_the_end() {
        // Round 8 (R8-9 / R8-R6): fold [A, T, C] —
        // A {children, CC BY, no credit}, T {title
        // only}, C {children, credit}. Pairwise judging
        // dropped the license at A+T, before C's credit
        // arrived to stand beside it. The fold now
        // judges the composed rule once, on the final
        // folded record, so both orders compose
        // identically: CC BY + credit + title stand.
        let credit_text = "Photo by Alice".to_string();
        let title_text = "Harvest supper".to_string();
        let a = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let t = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let c = RightsRecord {
            children_present: Some(true),
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let hash = sha256_hex(b"R8FOLD");
        let asset = |rights: RightsRecord| Asset {
            sha256: hash.clone(),
            kind: AssetKind::Picture,
            media_type: "image/png".into(),
            rights,
        };
        for records in [
            vec![a.clone(), t.clone(), c.clone()],
            vec![a.clone(), c.clone(), t.clone()],
        ] {
            let mut assets: Vec<Asset> = records.into_iter().map(asset).collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            let rights = &assets[0].rights;
            assert_eq!(rights.children_present, Some(true));
            assert_eq!(rights.license, LicenseKind::CcBy("4.0".into()));
            assert_eq!(rights.credit_text, credit_text);
            assert_eq!(rights.title, title_text);
        }
        // The same sequence through single merges
        // (add_asset) keeps the R7-A pairwise outcome
        // exactly: A+T drops the license before C's
        // credit arrives, and the credit then stands
        // beside no license.
        let mut doc = bare_doc();
        doc.add_asset(b"R8PIC", AssetKind::Picture, "image/png", a.clone());
        doc.add_asset(b"R8PIC", AssetKind::Picture, "image/png", t.clone());
        doc.add_asset(b"R8PIC", AssetKind::Picture, "image/png", c.clone());
        assert_eq!(doc.assets.len(), 1);
        let rights = &doc.assets[0].rights;
        assert_eq!(rights.children_present, Some(true));
        assert_eq!(rights.license, LicenseKind::Unknown);
        assert_eq!(rights.credit_text, credit_text);
        assert_eq!(rights.title, title_text);
    }

    #[test]
    fn r9_unvouched_licensed_reprint_falls_in_both_paths() {
        // Round 9 (R9-1 / RT9-1): a LicensedReprint
        // that IS the standing license, or that fills
        // the empty slot beside a children note, was
        // judged by nothing. One final judgment now
        // runs on every composed record: unvouched, it
        // falls to Unknown in both paths, in every
        // witness order.
        let lr = RightsRecord {
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let kids = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let single = |seq: &[RightsRecord]| -> RightsRecord {
            let mut acc = seq[0].clone();
            for next in &seq[1..] {
                acc = merge_rights_records(&acc, next);
            }
            acc
        };
        let folded = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R9FOLD");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        let witnesses: Vec<Vec<RightsRecord>> = vec![
            vec![lr.clone(), kids_ccby.clone()],
            vec![kids_ccby.clone(), lr.clone()],
            vec![kids.clone(), lr.clone()],
            vec![lr.clone(), kids.clone()],
            vec![kids_ccby.clone(), title_only.clone(), lr.clone()],
            vec![lr.clone(), kids_ccby.clone(), title_only.clone()],
        ];
        for seq in &witnesses {
            for composed in [single(seq), folded(seq)] {
                assert_eq!(composed.children_present, Some(true), "{seq:?}");
                assert_eq!(composed.credit_text, "", "{seq:?}");
                assert_eq!(composed.license, LicenseKind::Unknown, "{seq:?}");
            }
            // The two paths compose identically.
            let via_single = single(seq);
            let via_fold = folded(seq);
            assert_eq!(
                (
                    via_single.children_present,
                    via_single.license.clone(),
                    via_single.credit_text.clone()
                ),
                (
                    via_fold.children_present,
                    via_fold.license.clone(),
                    via_fold.credit_text.clone()
                ),
                "{seq:?}"
            );
        }
    }

    #[test]
    fn r9_vouched_licensed_reprint_controls_stand_in_every_order() {
        // Round 9 (R9-1): the vouched controls stand —
        // a record holding LicensedReprint together
        // with a children note, and a record holding
        // LicensedReprint together with its own credit
        // (that credit standing in the composed
        // record) — in every order, in both paths.
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let kids_lr = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let credit_text = "Photo by H".to_string();
        let kids_lr_credit = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let lr_credit = RightsRecord {
            license: LicenseKind::LicensedReprint,
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let kids_credit = RightsRecord {
            children_present: Some(true),
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let single = |seq: &[RightsRecord]| -> RightsRecord {
            let mut acc = seq[0].clone();
            for next in &seq[1..] {
                acc = merge_rights_records(&acc, next);
            }
            acc
        };
        let folded = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R9CTRL");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        // LicensedReprint + children, against the
        // attribution record, in both orders.
        for seq in [
            vec![kids_ccby.clone(), kids_lr.clone()],
            vec![kids_lr.clone(), kids_ccby.clone()],
        ] {
            for composed in [single(&seq), folded(&seq)] {
                assert_eq!(composed.license, LicenseKind::LicensedReprint, "{seq:?}");
                assert_eq!(composed.children_present, Some(true), "{seq:?}");
            }
        }
        // LicensedReprint + children + its own credit:
        // the credit stands, the license stands.
        for seq in [
            vec![kids_lr_credit.clone(), title_only.clone()],
            vec![title_only.clone(), kids_lr_credit.clone()],
        ] {
            for composed in [single(&seq), folded(&seq)] {
                assert_eq!(composed.license, LicenseKind::LicensedReprint, "{seq:?}");
                assert_eq!(composed.credit_text, credit_text, "{seq:?}");
            }
        }
        // LicensedReprint + its own credit, where the
        // same credit stands on a children-bearing
        // record in the composition: the credit that
        // stands is the credit its record held.
        for seq in [
            vec![lr_credit.clone(), kids_credit.clone()],
            vec![kids_credit.clone(), lr_credit.clone()],
        ] {
            for composed in [single(&seq), folded(&seq)] {
                assert_eq!(composed.license, LicenseKind::LicensedReprint, "{seq:?}");
                assert_eq!(composed.credit_text, credit_text, "{seq:?}");
                assert_eq!(composed.children_present, Some(true), "{seq:?}");
            }
        }
    }

    #[test]
    fn r9_fold_fallback_winner_is_order_independent() {
        // Round 9 (R9-4 / RT9-4): the fold's fallback
        // winner was the first vouched match in fold
        // order. Fallback selection is order-independent
        // now: a vouched license carried by a
        // children-bearing source record beats one
        // vouched only through the bearer rule, and
        // among equals the fixed precedence decides —
        // the church's own work, then public domain,
        // then CC0, then a reprint license.
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let kids_cc0 = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let church = RightsRecord {
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let kids_pd = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::PublicDomain,
            ..Default::default()
        };
        let kids_church = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::ChurchOwned,
            ..Default::default()
        };
        let kids_lr = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let folded_license = |seq: &[RightsRecord]| -> LicenseKind {
            let hash = sha256_hex(b"R9FALL");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.license.clone()
        };
        // The RT9-4 witness pair: Cc0 is carried by a
        // children-bearing record; ChurchOwned is
        // vouched only through the bearer rule (the
        // Cc0 record). Cc0 wins in both orders.
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_cc0.clone(), church.clone()]),
            LicenseKind::Cc0
        );
        assert_eq!(
            folded_license(&[kids_ccby.clone(), church.clone(), kids_cc0.clone()]),
            LicenseKind::Cc0
        );
        // Both carriers children-bearing: the fixed
        // precedence decides — ChurchOwned over
        // PublicDomain, in both orders.
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_pd.clone(), kids_church.clone()]),
            LicenseKind::ChurchOwned
        );
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_church.clone(), kids_pd.clone()]),
            LicenseKind::ChurchOwned
        );
        // Both carriers children-bearing: CC0 over a
        // reprint license, in both orders.
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_lr.clone(), kids_cc0.clone()]),
            LicenseKind::Cc0
        );
        assert_eq!(
            folded_license(&[kids_ccby.clone(), kids_cc0.clone(), kids_lr.clone()]),
            LicenseKind::Cc0
        );
    }

    #[test]
    fn r9_fold_rejudges_the_assertion_against_the_final_record() {
        // Round 9 (R9-5 / RT9-5): in a fold, an
        // assertion rode in on a license the fold-end
        // judge then dropped, and the assertion stayed.
        // At fold end the assertion is re-judged by
        // the R7-8 rule against the FINAL folded
        // record: it stands only if a substantive
        // rights fill from its own record stands there.
        let empty = RightsRecord::default();
        let asserter_name = "Mallory".to_string();
        let asserter_date = "2026-01-01".to_string();
        let kids_ccby_asserted = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            asserted_by: asserter_name.clone(),
            asserted_at: asserter_date.clone(),
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let folded = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R9ASSERT");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        for seq in [
            vec![empty.clone(), kids_ccby_asserted.clone()],
            vec![
                title_only.clone(),
                kids_ccby_asserted.clone(),
                title_only.clone(),
            ],
        ] {
            let composed = folded(&seq);
            assert_eq!(composed.license, LicenseKind::Unknown, "{seq:?}");
            assert_eq!(composed.asserted_by, "", "{seq:?}");
            assert_eq!(composed.asserted_at, "", "{seq:?}");
        }
        // The single-merge control is unchanged: the
        // same pair through one merge leaves the
        // assertion empty too.
        let merged = merge_rights_records(&empty, &kids_ccby_asserted);
        assert_eq!(merged.license, LicenseKind::Unknown);
        assert_eq!(merged.asserted_by, "");
        assert_eq!(merged.asserted_at, "");
        // Control: when the asserting record's own
        // credit stands in the final folded record,
        // the assertion stands with it.
        let credit_text = "Photo by Alice".to_string();
        let kids_credit_asserted = RightsRecord {
            children_present: Some(true),
            credit_text: credit_text.clone(),
            asserted_by: asserter_name.clone(),
            asserted_at: asserter_date.clone(),
            ..Default::default()
        };
        let composed = folded(&[empty.clone(), kids_credit_asserted]);
        assert_eq!(composed.credit_text, credit_text);
        assert_eq!(composed.asserted_by, asserter_name);
        assert_eq!(composed.asserted_at, asserter_date);
    }

    #[test]
    fn r10_fold_and_single_merge_routes_agree_over_the_pool() {
        // Round 10 (R10-2, red team RT10-R1): the
        // fold route and the single-merge route used
        // to pick different licenses for the same
        // records — 23,823 of 346,176 sequences of
        // length <= 4 over this pool in the round-10
        // lane's brute force. Every sampled divergence
        // had the same shape: a credit line standing
        // in the composition, an attribution license
        // traveling with that credit, and a different
        // license standing. The license that travels
        // with the credit that stands now wins on
        // both routes (see
        // `credit_paired_attribution_license`), and
        // this pin holds the property in three parts
        // over the lane's 24-archetype pool (27
        // archetypes since the R11-A widening below):
        //
        // (1) The rule itself, on both routes, over
        // every sequence of lengths 2 and 3: judged
        // against the sources each route's final
        // judgment sees (the fold: the whole
        // sequence; a single merge: its last pair),
        // whenever an attribution license in the
        // composition is vouched by the standing
        // credit and the standing license is not, the
        // vouched license stands.
        //
        // (2) Full (children, license, credit)
        // agreement between the routes: over every
        // length-2 sequence in the pool, and over
        // every sequence of lengths 2 to 4 in the
        // paired family below — the archetypes
        // whose credit lines travel with an
        // attribution license, or carry no credit at
        // all, which is the family the finding lived
        // in. Outside that family the routes are
        // still pinned to differ, by design, where a
        // credit arrives only after the license's
        // pairwise judgment and travels with no
        // attribution license (round 8, R8-9, pinned
        // in
        // `r8_fold_judges_the_composed_rule_once_at_the_end`):
        // there no attribution license is vouched by
        // the standing credit, and the R10-2 rule
        // leaves today's judgment unchanged.
        //
        // (3) The round-9 brute-force invariants on
        // every folded record: no unvouched license
        // beside children with an empty credit line,
        // no manufactured credit or license, no
        // orphan assertion.
        //
        // Round 11 (R11-A) widens the pool and the
        // pin. The pool gains children-bearing
        // records carrying fallback-class licenses
        // WITH credit lines (CC0, public domain, and
        // the church's own work, each beside a
        // children note and a credit) — the paired
        // family below used to exclude every kids
        // record with a fallback-class license, the
        // blind spot the R11-5 witness lived in. And
        // `check_seq` now asserts the safety axes on
        // EVERY sequence, not only where the routes
        // fully agree: the children note and
        // `rights_permission_noted` (the web page's
        // hold-back bit) must agree between the
        // routes with zero tolerance, at every
        // length the pin runs, over the full pool
        // and over a widened family that carries the
        // new archetypes at lengths 2 to 4.
        fn arch(children: Option<bool>, license: LicenseKind, credit: &str) -> RightsRecord {
            RightsRecord {
                children_present: children,
                license,
                credit_text: credit.into(),
                ..Default::default()
            }
        }
        let ccby = || LicenseKind::CcBy("4.0".into());
        let ccbysa = || LicenseKind::CcBySa("4.0".into());
        let mut pool: Vec<RightsRecord> = vec![
            arch(None, LicenseKind::Unknown, ""),
            arch(Some(true), LicenseKind::Unknown, ""),
            arch(Some(true), ccby(), ""),
            arch(Some(true), ccby(), "Photo by A"),
            arch(None, ccby(), ""),
            arch(None, ccby(), "Photo by B"),
            arch(Some(true), LicenseKind::Unknown, "Photo by C"),
            arch(None, LicenseKind::Unknown, "Photo by D"),
            arch(Some(true), LicenseKind::Cc0, ""),
            arch(None, LicenseKind::Cc0, ""),
            arch(Some(true), LicenseKind::PublicDomain, ""),
            arch(None, LicenseKind::PublicDomain, ""),
            arch(Some(true), LicenseKind::ChurchOwned, ""),
            arch(None, LicenseKind::ChurchOwned, ""),
            arch(Some(true), LicenseKind::LicensedReprint, ""),
            arch(None, LicenseKind::LicensedReprint, ""),
            arch(Some(true), ccbysa(), ""),
            arch(None, ccbysa(), "Photo by E"),
            arch(Some(false), ccby(), "Photo by F"),
            arch(Some(true), ccbysa(), "Photo by G"),
            arch(Some(false), LicenseKind::Cc0, ""),
            arch(Some(true), LicenseKind::LicensedReprint, "Photo by H"),
        ];
        pool.push(RightsRecord {
            title: "Harvest supper".into(),
            ..Default::default()
        });
        pool.push(RightsRecord {
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..arch(Some(true), ccby(), "")
        });
        // The R11-A widening (indices 24 to 26):
        // children-bearing records carrying
        // fallback-class licenses with credit lines.
        pool.push(arch(Some(true), LicenseKind::Cc0, "Photo by I"));
        pool.push(arch(Some(true), LicenseKind::PublicDomain, "Photo by J"));
        pool.push(arch(Some(true), LicenseKind::ChurchOwned, "Photo by K"));
        let fold_route = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R10ROUTES");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        // The single-merge route: the public
        // pairwise merge, in the same order, starting
        // from the first record — and the record
        // standing before its last merge, whose pair
        // is what its final judgment sees.
        let single_route = |seq: &[RightsRecord]| -> (RightsRecord, RightsRecord) {
            let mut acc = seq[0].clone();
            let mut before_last = seq[0].clone();
            for (i, next) in seq.iter().enumerate().skip(1) {
                if i + 1 == seq.len() {
                    before_last = acc.clone();
                }
                acc = merge_rights_records(&acc, next);
            }
            (acc, before_last)
        };
        let triple =
            |r: &RightsRecord| (r.children_present, r.license.clone(), r.credit_text.clone());
        // The R10-2 rule, stated as a check on one
        // route's outcome against the sources its
        // final judgment saw.
        let rule_holds = |out: &RightsRecord, sources: &[&RightsRecord]| -> bool {
            if out.credit_text.trim().is_empty() {
                return true;
            }
            let paired: Vec<&LicenseKind> = sources
                .iter()
                .filter(|r| {
                    r.credit_text == out.credit_text
                        && matches!(r.license, LicenseKind::CcBy(_) | LicenseKind::CcBySa(_))
                })
                .map(|r| &r.license)
                .collect();
            if paired.is_empty() {
                return true;
            }
            let standing_paired = out.license != LicenseKind::Unknown
                && sources
                    .iter()
                    .any(|r| r.license == out.license && r.credit_text == out.credit_text);
            standing_paired || paired.iter().any(|p| **p == out.license)
        };
        let check_seq = |seq: &[RightsRecord], agree: bool| {
            let folded = fold_route(seq);
            let (merged, before_last) = single_route(seq);
            // The safety axes (round 11, R11-1), on
            // every sequence regardless of `agree`:
            // the children note and whether a
            // permission stands beside it are
            // route-invariant, zero tolerance.
            assert_eq!(
                (folded.children_present, rights_permission_noted(&folded)),
                (merged.children_present, rights_permission_noted(&merged)),
                "safety axes disagree on {seq:?}: fold {folded:?} vs single {merged:?}"
            );
            if agree {
                assert_eq!(
                    triple(&folded),
                    triple(&merged),
                    "routes disagree on {seq:?}"
                );
            }
            let fold_sources: Vec<&RightsRecord> = seq.iter().collect();
            assert!(
                rule_holds(&folded, &fold_sources),
                "the credit-paired license does not stand on the fold route: {seq:?} -> {folded:?}"
            );
            let last = seq.last().expect("a sequence of two or more");
            let single_sources: [&RightsRecord; 2] = [&before_last, last];
            assert!(
                rule_holds(&merged, &single_sources),
                "the credit-paired license does not stand on the single-merge route: {seq:?} -> {merged:?}"
            );
            // The round-9 invariants on the folded
            // record (where the routes agree, they
            // hold for the merged record too).
            let sources = &fold_sources;
            if folded.children_present == Some(true) {
                if folded.credit_text.trim().is_empty() && folded.license != LicenseKind::Unknown {
                    assert!(
                        license_vouched(&folded.license, &folded.credit_text, sources),
                        "unvouched license stands beside children: {seq:?} -> {folded:?}"
                    );
                }
                let bearer = sources
                    .iter()
                    .any(|r| r.children_present == Some(true) && rights_permission_noted(r));
                if !folded.credit_text.trim().is_empty() {
                    let credit_from_kids = sources.iter().any(|r| {
                        r.credit_text == folded.credit_text && r.children_present == Some(true)
                    });
                    assert!(
                        credit_from_kids || bearer,
                        "manufactured credit: {seq:?} -> {folded:?}"
                    );
                }
                if matches!(
                    folded.license,
                    LicenseKind::ChurchOwned | LicenseKind::PublicDomain | LicenseKind::Cc0
                ) {
                    let license_from_kids = sources
                        .iter()
                        .any(|r| r.license == folded.license && r.children_present == Some(true));
                    assert!(
                        license_from_kids || bearer,
                        "manufactured license: {seq:?} -> {folded:?}"
                    );
                }
            }
            if !folded.asserted_by.trim().is_empty() || !folded.asserted_at.trim().is_empty() {
                let supplier_stands = sources.iter().any(|r| {
                    !r.asserted_by.trim().is_empty()
                        && r.asserted_by == folded.asserted_by
                        && (r.license == folded.license
                            || (!r.credit_text.trim().is_empty()
                                && r.credit_text == folded.credit_text))
                });
                assert!(supplier_stands, "orphan assertion: {seq:?} -> {folded:?}");
            }
        };
        let sequences_over = |pool: &[RightsRecord], len: usize| -> Vec<Vec<RightsRecord>> {
            let n = pool.len() as u64;
            let count = n.pow(len as u32);
            let mut out = Vec::with_capacity(count as usize);
            for mut code in 0..count {
                let mut seq: Vec<RightsRecord> = Vec::with_capacity(len);
                for _ in 0..len {
                    seq.push(pool[(code % n) as usize].clone());
                    code /= n;
                }
                out.push(seq);
            }
            out
        };
        // (1) and (3) over the full pool, lengths 2
        // and 3; (2) in full at length 2.
        for len in 2usize..=3 {
            for seq in sequences_over(&pool, len) {
                check_seq(&seq, len == 2);
            }
        }
        // (2) over the paired family, lengths 2 to
        // 4: the kids record, the attribution
        // licenses with and without their credits,
        // and the paired credit records.
        let family: Vec<RightsRecord> = [1usize, 2, 3, 4, 5, 16, 17, 19]
            .iter()
            .map(|&i| pool[i].clone())
            .collect();
        for len in 2usize..=4 {
            for seq in sequences_over(&family, len) {
                check_seq(&seq, true);
            }
        }
        // The widened family (round 11, R11-A):
        // children-bearing records carrying
        // fallback-class licenses, with and without
        // credit lines, beside the attribution
        // records — the shapes the R11-1 and R11-5
        // witnesses live in. Full triple agreement
        // is NOT asserted here (the license-only
        // deferral residual lives in this family
        // too); the safety axes, the credit-paired
        // rule, and the invariants are, on every
        // sequence, lengths 2 to 4.
        let widened_family: Vec<RightsRecord> = [1usize, 2, 5, 8, 10, 14, 24, 25]
            .iter()
            .map(|&i| pool[i].clone())
            .collect();
        for len in 2usize..=4 {
            for seq in sequences_over(&widened_family, len) {
                check_seq(&seq, false);
            }
        }
        // The lane's named shape as a readable
        // witness beside the brute force: two
        // attribution licenses and the credit record
        // — the credit record's license stands on
        // both routes, in both orders.
        let first_license = arch(Some(true), ccbysa(), "");
        let credit_record = arch(Some(true), ccby(), "Photo by A");
        for seq in [
            vec![first_license.clone(), credit_record.clone()],
            vec![credit_record.clone(), first_license.clone()],
        ] {
            let expected = (Some(true), ccby(), "Photo by A".to_string());
            assert_eq!(triple(&fold_route(&seq)), expected, "fold witness {seq:?}");
            let (merged, _) = single_route(&seq);
            assert_eq!(triple(&merged), expected, "single witness {seq:?}");
        }
    }

    #[test]
    fn r11_permission_axis_and_credit_witnesses_agree_on_both_routes() {
        // Round 11 (R11-A, review R11-1 and red team
        // RT11-7): the two named witnesses, pinned on
        // both routes with their exact outcomes.
        //
        // R11-1 witness 1: [LicensedReprint, {children},
        // {children, Cc0}] folded to not-noted (the
        // picture held back) and single-merged to
        // noted (it rode the web page). The shared
        // judgment's permission step now stands the
        // vouched permission-noting license (Cc0) on
        // both routes.
        //
        // R11-1 witness 2: [{children, CcBy},
        // {children, LicensedReprint}, {children,
        // Cc0}] folded to noted and single-merged to
        // not-noted; both routes now end at Cc0.
        //
        // R11-5 witness: [{children, CcBy},
        // {children, Cc0}, {CcBy, "Photo by B"}]
        // folded to (children, Cc0, "") — the
        // composition's only credit silently dropped
        // on the load route — and single-merged to
        // (children, CcBy, "Photo by B"). The fold-end
        // restoration now stands the vouched credit,
        // and its paired license beside it, on the
        // fold route too.
        fn arch(children: Option<bool>, license: LicenseKind, credit: &str) -> RightsRecord {
            RightsRecord {
                children_present: children,
                license,
                credit_text: credit.into(),
                ..Default::default()
            }
        }
        let ccby = || LicenseKind::CcBy("4.0".into());
        let fold_route = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R11WIT");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        let single_route = |seq: &[RightsRecord]| -> RightsRecord {
            let mut acc = seq[0].clone();
            for next in &seq[1..] {
                acc = merge_rights_records(&acc, next);
            }
            acc
        };
        let triple =
            |r: &RightsRecord| (r.children_present, r.license.clone(), r.credit_text.clone());
        let kids_cc0 = (Some(true), LicenseKind::Cc0, String::new());
        let witness1 = vec![
            arch(None, LicenseKind::LicensedReprint, ""),
            arch(Some(true), LicenseKind::Unknown, ""),
            arch(Some(true), LicenseKind::Cc0, ""),
        ];
        assert_eq!(triple(&fold_route(&witness1)), kids_cc0, "fold witness 1");
        assert_eq!(
            triple(&single_route(&witness1)),
            kids_cc0,
            "single witness 1"
        );
        assert!(rights_permission_noted(&fold_route(&witness1)));
        let witness2 = vec![
            arch(Some(true), ccby(), ""),
            arch(Some(true), LicenseKind::LicensedReprint, ""),
            arch(Some(true), LicenseKind::Cc0, ""),
        ];
        assert_eq!(triple(&fold_route(&witness2)), kids_cc0, "fold witness 2");
        assert_eq!(
            triple(&single_route(&witness2)),
            kids_cc0,
            "single witness 2"
        );
        assert!(rights_permission_noted(&single_route(&witness2)));
        let credit_witness = vec![
            arch(Some(true), ccby(), ""),
            arch(Some(true), LicenseKind::Cc0, ""),
            arch(None, ccby(), "Photo by B"),
        ];
        let with_credit = (Some(true), ccby(), "Photo by B".to_string());
        assert_eq!(
            triple(&fold_route(&credit_witness)),
            with_credit,
            "fold credit witness"
        );
        assert_eq!(
            triple(&single_route(&credit_witness)),
            with_credit,
            "single credit witness"
        );
    }

    #[test]
    fn r11_shared_credit_pairings_follow_the_precedence_on_both_routes() {
        // Round 11 (R11-9, review R11-2): two
        // attribution licenses carrying the SAME
        // credit text — the fold used to pick the
        // first pairing in fold order, a chain of
        // single merges another (132 divergences in
        // the review lane's shared-credit pool). The
        // pairing is now chosen by the standing
        // precedence (CC BY before CC BY-SA, then
        // version) among the pairings the routes'
        // merges actually saw, identically on both
        // routes. The named shapes below are pinned
        // to that tie-break, and the shared-credit
        // pool they live in joins the pin pool: over
        // every sequence of lengths 2 to 4 the two
        // routes agree on the full triple.
        fn arch(children: Option<bool>, license: LicenseKind, credit: &str) -> RightsRecord {
            RightsRecord {
                children_present: children,
                license,
                credit_text: credit.into(),
                ..Default::default()
            }
        }
        let ccby = || LicenseKind::CcBy("4.0".into());
        let ccbysa = || LicenseKind::CcBySa("4.0".into());
        let fold_route = |seq: &[RightsRecord]| -> RightsRecord {
            let hash = sha256_hex(b"R11SHARED");
            let mut assets: Vec<Asset> = seq
                .iter()
                .cloned()
                .map(|rights| Asset {
                    sha256: hash.clone(),
                    kind: AssetKind::Picture,
                    media_type: "image/png".into(),
                    rights,
                })
                .collect();
            fold_duplicate_asset_records(&mut assets);
            assert_eq!(assets.len(), 1);
            assets[0].rights.clone()
        };
        let single_route = |seq: &[RightsRecord]| -> RightsRecord {
            let mut acc = seq[0].clone();
            for next in &seq[1..] {
                acc = merge_rights_records(&acc, next);
            }
            acc
        };
        let triple =
            |r: &RightsRecord| (r.children_present, r.license.clone(), r.credit_text.clone());
        let by_x = (Some(true), ccby(), "Photo by X".to_string());
        let bysa_x = (Some(true), ccbysa(), "Photo by X".to_string());
        // The review lane's witness: the fold took
        // the first pairing (CC BY-SA), the single
        // merges the later one (CC BY). The
        // precedence winner (CC BY) now stands on
        // both routes.
        let review_witness = vec![
            arch(Some(true), LicenseKind::LicensedReprint, ""),
            arch(None, ccbysa(), "Photo by X"),
            arch(Some(true), ccby(), "Photo by X"),
        ];
        assert_eq!(
            triple(&fold_route(&review_witness)),
            by_x,
            "fold review witness"
        );
        assert_eq!(
            triple(&single_route(&review_witness)),
            by_x,
            "single review witness"
        );
        // The mirror order: the precedence winner
        // arrives first and stands on both routes.
        let mirror_witness = vec![
            arch(Some(true), ccby(), "Photo by X"),
            arch(None, ccbysa(), "Photo by X"),
            arch(Some(true), LicenseKind::LicensedReprint, ""),
        ];
        assert_eq!(
            triple(&fold_route(&mirror_witness)),
            by_x,
            "fold mirror witness"
        );
        assert_eq!(
            triple(&single_route(&mirror_witness)),
            by_x,
            "single mirror witness"
        );
        // A pairing whose credit a later merge
        // reverts (the children note arrives with no
        // permission noted) no longer counts on
        // either route: the pairing that re-supplies
        // the credit stands alone.
        let reverted_pairing = vec![
            arch(None, ccby(), "Photo by X"),
            arch(None, ccbysa(), "Photo by X"),
            arch(Some(true), LicenseKind::LicensedReprint, ""),
            arch(Some(true), ccbysa(), "Photo by X"),
        ];
        assert_eq!(
            triple(&fold_route(&reverted_pairing)),
            bysa_x,
            "fold reverted pairing"
        );
        assert_eq!(
            triple(&single_route(&reverted_pairing)),
            bysa_x,
            "single reverted pairing"
        );
        // The bearer shape: a permission-noting
        // children record stands before both
        // pairings, so each pairing's credit stands
        // at its arrival on both routes and the
        // precedence chooses between the two.
        let bearer_shape = vec![
            arch(Some(true), LicenseKind::LicensedReprint, ""),
            arch(Some(true), LicenseKind::Cc0, ""),
            arch(None, ccby(), "Photo by X"),
            arch(None, ccbysa(), "Photo by X"),
        ];
        assert_eq!(
            triple(&fold_route(&bearer_shape)),
            by_x,
            "fold bearer shape"
        );
        assert_eq!(
            triple(&single_route(&bearer_shape)),
            by_x,
            "single bearer shape"
        );
        // The dropped-pairing shape: the first
        // pairing's credit is reverted at its
        // arrival (the children note arrives with no
        // permission noted beside it), so a chain of
        // single merges drops that pairing there and
        // the second pairing — carried by a
        // children-bearing record — stands alone;
        // the fold route answers the same.
        let dropped_pairing = vec![
            arch(None, ccby(), "Photo by X"),
            arch(Some(true), LicenseKind::LicensedReprint, ""),
            arch(Some(true), ccbysa(), "Photo by X"),
        ];
        assert_eq!(
            triple(&fold_route(&dropped_pairing)),
            bysa_x,
            "fold dropped pairing"
        );
        assert_eq!(
            triple(&single_route(&dropped_pairing)),
            bysa_x,
            "single dropped pairing"
        );
        // Both pairings stand before any children
        // note arrives, and a children-bearing
        // fallback record vouches the credit: the
        // precedence winner of the two stands on
        // both routes.
        let vouched_pairings = vec![
            arch(None, ccbysa(), "Photo by X"),
            arch(None, ccby(), "Photo by X"),
            arch(Some(true), LicenseKind::Cc0, ""),
        ];
        assert_eq!(
            triple(&fold_route(&vouched_pairings)),
            by_x,
            "fold vouched pairings"
        );
        assert_eq!(
            triple(&single_route(&vouched_pairings)),
            by_x,
            "single vouched pairings"
        );
        // The shared-credit pool joins the pin pool:
        // full triple agreement between the routes
        // over every sequence of lengths 2 to 4.
        let pool: Vec<RightsRecord> = vec![
            arch(Some(true), LicenseKind::LicensedReprint, ""),
            arch(None, ccbysa(), "Photo by X"),
            arch(Some(true), ccby(), "Photo by X"),
            arch(Some(true), LicenseKind::Unknown, ""),
            arch(None, ccby(), "Photo by X"),
            arch(Some(true), ccbysa(), "Photo by X"),
            arch(Some(true), LicenseKind::Cc0, ""),
            arch(None, LicenseKind::Unknown, ""),
            arch(Some(false), ccbysa(), "Photo by X"),
        ];
        let sequences_over = |pool: &[RightsRecord], len: usize| -> Vec<Vec<RightsRecord>> {
            let n = pool.len() as u64;
            let count = n.pow(len as u32);
            let mut out = Vec::with_capacity(count as usize);
            for mut code in 0..count {
                let mut seq: Vec<RightsRecord> = Vec::with_capacity(len);
                for _ in 0..len {
                    seq.push(pool[(code % n) as usize].clone());
                    code /= n;
                }
                out.push(seq);
            }
            out
        };
        for len in 2usize..=4 {
            for seq in sequences_over(&pool, len) {
                assert_eq!(
                    triple(&fold_route(&seq)),
                    triple(&single_route(&seq)),
                    "shared-credit routes disagree on {seq:?}"
                );
            }
        }
    }

    #[test]
    fn r9_fold_triple_is_permutation_invariant_over_the_pool() {
        // Round 9 (R9-1): a brute-force-style
        // permutation check, in-tree over a small
        // archetype pool: for each multiset tested,
        // every permutation folds to the same
        // (children, license, credit) triple.
        fn all_perms(idxs: &[usize]) -> Vec<Vec<usize>> {
            if idxs.len() <= 1 {
                return vec![idxs.to_vec()];
            }
            let mut out = Vec::new();
            for i in 0..idxs.len() {
                let mut rest = idxs.to_vec();
                let first = rest.remove(i);
                for tail in all_perms(&rest) {
                    let mut perm = vec![first];
                    perm.extend(tail);
                    out.push(perm);
                }
            }
            out
        }
        let lr = RightsRecord {
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let kids = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let kids_ccby = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::CcBy("4.0".into()),
            ..Default::default()
        };
        let kids_cc0 = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::Cc0,
            ..Default::default()
        };
        let kids_lr = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            ..Default::default()
        };
        let title_text = "Harvest supper".to_string();
        let title_only = RightsRecord {
            title: title_text.clone(),
            ..Default::default()
        };
        let credit_text = "Photo by H".to_string();
        let kids_lr_credit = RightsRecord {
            children_present: Some(true),
            license: LicenseKind::LicensedReprint,
            credit_text: credit_text.clone(),
            ..Default::default()
        };
        // (multiset, the triple every permutation
        // must fold to). Each multiset tested here
        // carries at most one vouched non-attribution
        // license, so the final judgment fully
        // determines the triple: the witness multisets
        // (no vouched license at all), the vouched
        // controls (one), and single-candidate
        // fallback multisets (one). Multisets with
        // two vouched licenses are pinned by their
        // RT9-4 witness orders in
        // `r9_fold_fallback_winner_is_order_independent`
        // — there the standing license, when it is
        // itself vouched, stands under the fill law,
        // and precedence governs only the fallback a
        // fallen license drops to.
        type Triple = (Option<bool>, LicenseKind, String);
        let cases: Vec<(Vec<RightsRecord>, Triple)> = vec![
            (
                vec![lr.clone(), kids_ccby.clone()],
                (Some(true), LicenseKind::Unknown, String::new()),
            ),
            (
                vec![kids.clone(), lr.clone()],
                (Some(true), LicenseKind::Unknown, String::new()),
            ),
            (
                vec![kids_ccby.clone(), title_only.clone(), lr.clone()],
                (Some(true), LicenseKind::Unknown, String::new()),
            ),
            (
                vec![kids_ccby.clone(), kids_lr.clone()],
                (Some(true), LicenseKind::LicensedReprint, String::new()),
            ),
            (
                vec![kids_ccby.clone(), kids_cc0.clone(), title_only.clone()],
                (Some(true), LicenseKind::Cc0, String::new()),
            ),
            (
                vec![kids_ccby.clone(), kids_lr.clone(), title_only.clone()],
                (Some(true), LicenseKind::LicensedReprint, String::new()),
            ),
            (
                vec![kids_lr_credit.clone(), title_only.clone()],
                (
                    Some(true),
                    LicenseKind::LicensedReprint,
                    credit_text.clone(),
                ),
            ),
        ];
        for (multiset, expected) in &cases {
            let indices: Vec<usize> = (0..multiset.len()).collect();
            for perm in all_perms(&indices) {
                let seq: Vec<RightsRecord> = perm.iter().map(|&i| multiset[i].clone()).collect();
                let hash = sha256_hex(b"R9PERM");
                let mut assets: Vec<Asset> = seq
                    .iter()
                    .cloned()
                    .map(|rights| Asset {
                        sha256: hash.clone(),
                        kind: AssetKind::Picture,
                        media_type: "image/png".into(),
                        rights,
                    })
                    .collect();
                fold_duplicate_asset_records(&mut assets);
                assert_eq!(assets.len(), 1);
                let rights = &assets[0].rights;
                let triple = (
                    rights.children_present,
                    rights.license.clone(),
                    rights.credit_text.clone(),
                );
                assert_eq!(&triple, expected, "permutation {perm:?} of {multiset:?}");
            }
        }
    }

    #[test]
    fn r7_add_asset_in_the_mirror_order_drops_the_license() {
        // Round 7 (R7-1): add_asset in the mirror
        // order — the bulletin first holds the picture
        // as children-noted with a CC BY license, then
        // the same bytes arrive again carrying only
        // the credit the fence refuses.
        let mut doc = bare_doc();
        let first = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            children_present: Some(true),
            ..Default::default()
        };
        let second = RightsRecord {
            credit_text: "Photo by Alice, CC BY 4.0".into(),
            ..Default::default()
        };
        doc.add_asset(b"R7PIC", AssetKind::Picture, "image/png", first);
        doc.add_asset(b"R7PIC", AssetKind::Picture, "image/png", second);
        assert_eq!(doc.assets.len(), 1);
        let rights = &doc.assets[0].rights;
        assert_eq!(rights.children_present, Some(true));
        assert_eq!(rights.license, LicenseKind::Unknown);
        assert_eq!(rights.credit_text, "");
    }

    #[test]
    fn r7_assertion_rides_only_a_standing_rights_fill() {
        // Round 7 (R7-8 / RT7-8, the probe verbatim):
        // the incoming record's credit is refused by
        // the fence, its title fill lands — and its
        // assertion used to ride in on that unrelated
        // fill, reading as vouching for a composed
        // record its author never saw.
        let existing = RightsRecord {
            children_present: Some(true),
            ..Default::default()
        };
        let title_fill = RightsRecord {
            title: "Harvest supper".into(),
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &title_fill);
        assert_eq!(merged.title, "Harvest supper");
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.asserted_by, "");
        assert_eq!(merged.asserted_at, "");
        // The same ride on a source_url-only fill.
        let url_fill = RightsRecord {
            source_url: Some("https://example.org/picnic".into()),
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&existing, &url_fill);
        assert_eq!(
            merged.source_url.as_deref(),
            Some("https://example.org/picnic")
        );
        assert_eq!(merged.credit_text, "");
        assert_eq!(merged.asserted_by, "");
        assert_eq!(merged.asserted_at, "");
        // The R5-A control stays held: an incoming
        // record carrying only the refused credit plus
        // the assertion changes nothing at all.
        let pair_only = RightsRecord {
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        assert_eq!(merge_rights_records(&existing, &pair_only), existing);
        // When the same record's credit genuinely
        // stands, the assertion stands with it.
        let plain = RightsRecord::default();
        let credited = RightsRecord {
            credit_text: "Photo by Alice".into(),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&plain, &credited);
        assert_eq!(merged.credit_text, "Photo by Alice");
        assert_eq!(merged.asserted_by, "Mallory");
        assert_eq!(merged.asserted_at, "2026-01-01");
        // A license that itself stands is substantive
        // the same way.
        let licensed = RightsRecord {
            license: LicenseKind::CcBy("4.0".into()),
            asserted_by: "Mallory".into(),
            asserted_at: "2026-01-01".into(),
            ..Default::default()
        };
        let merged = merge_rights_records(&plain, &licensed);
        assert_eq!(merged.license, LicenseKind::CcBy("4.0".into()));
        assert_eq!(merged.asserted_by, "Mallory");
        assert_eq!(merged.asserted_at, "2026-01-01");
    }
}
