# Template Analyzer (onboarding)

Jacob, 2026-10-15: "we have 10+ bulletins.. analyze the layout and for
onboarding use those together to generate a template suggestion that will
auto build a suggested template from what they provided, and then allow
the person to accept, edit, or cancel. Accept takes the template as is,
cancel closes it out, edit brings them to the template editor with that
template suggestion loaded."

## Why a corpus beats a single document
One PDF cannot tell you what is standing content and what is weekly: a
date and a service time look identical on the page. Ten PDFs can. Blocks
that appear in (nearly) every bulletin at the same place in the same style
are standing structure; blocks whose text changes every week are weekly
slots; text that changes in a pattern (dates, numbers, names) is a field
candidate. The corpus turns classification into statistics instead of
guesses, which is what makes the suggestion trustworthy enough to offer
one-click Accept.

## Flow (all local; nothing is uploaded anywhere)
1. Onboarding step: "Bring your old bulletins." Drop 3+ files (10+ is
   better), in any combination: PDFs, Publisher .pub files, InDesign
   IDML exports, and native .obd files. Mixed corpora are welcome and
   actively help (see below).
2. Extract per document into one shared intermediate form: panels,
   blocks (position, font, size, weight, text), images. Each import
   track (PDF, .pub rescue, IDML, .obd) already produces this form, so
   the analyzer sits on top of the importers instead of duplicating them.
   Structured sources carry real frames and styles, so they anchor the
   consensus; PDFs contribute exact appearance and geometry. When both
   exist for the same bulletin, the pair cross-checks the extraction.
3. Align panels across documents; cluster blocks by position + style.
4. Classify: standing (same text everywhere), weekly (text varies),
   field-like (varies by pattern: dates advance, hymn numbers change,
   readings rotate). Infer the fold preset from page geometry consensus.
5. Build a draft .obd template: panels, frames, stories with classified
   roles, styles mapped from font clusters, fields proposed for the
   pattern-varying blocks, church profile harvested from the cover.
6. Review screen, plain language: "Here is the template we built from
   your last 12 bulletins" with the preview and three buttons:
   - Accept: saves it as the church's template, onboarding continues.
   - Edit: opens the template editor with the suggestion loaded.
   - Cancel: closes with nothing saved; the built-in templates remain.
   Low-confidence items are listed in words ("We were not sure about the
   block at the bottom of page 2") and highlighted in the preview; they
   never silently become standing content.

## Guardrails
- A suggestion is a draft until a human accepts or edits it. Nothing is
  auto-applied to a real bulletin.
- Seasonal variants (Christmas, Easter) cluster separately instead of
  polluting the ordinary-Sunday template; the analyzer says so when it
  sees them ("2 of these look like special services; we set them aside").
- Works on the user's own files, on their machine. No cloud, no telemetry.
