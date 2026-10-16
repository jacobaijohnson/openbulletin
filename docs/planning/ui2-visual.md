# UI Audit, round 2 — Visual Design & Consistency (blind Wonder lane, 2026-10-15)

Read-only audit of crates/ob-app/src/main.rs (5,943 lines, HEAD
0eaeea3). The GUI was not run; every finding is grounded in the
code that produces the visuals. Summary verdict: the words are
excellent and the structure is right, but the app has almost no
visual system.

## BLOCKER

**B1. The step sidebar never scrolls, so primary actions fall off
the bottom of the window.** main.rs:4350–4354: steps live in
SidePanel::left with no ScrollArea in the panel (verified by grep;
scroll areas exist only in windows and the center preview, 4824).
egui panels clip overflow; clipped widgets are unreachable. Start
(4381–4560) holds ~1,100–1,200px of content in a ~760px interior
at the default 1180×820 window. Print is worst (4654–4819): every
sheet side is drawn at full sidebar width first (sheet_preview_ui
1807–1825; draw_sheet scales to 560px wide, a bifold ≈ 900px of
preview), and only then the checklist, the copy buttons, the
duplex wizard, and "Start next Sunday" (4814). The status/error
line renders at the bottom of the same sidebar (4808–4820),
including the fail() banner (531–534): the app's only feedback
channel is itself in the clipped zone on the two longest steps.
Fix: wrap step content (below the step list) in
ScrollArea::vertical() as the center panel does at 4824; on Print,
also cap the preview height or move the copy buttons above the
preview.

## MAJOR

**M1. No button anywhere is visually primary; Print emphasis
contradicts itself.** Zero .fill( calls in the app; the only
Button::new uses are enable-state wrappers. Top-bar "Next:" is
bold text on a gray button (4335–4342). Print (4701–4775): "Open
the print copy" plain, "Save print copy (PDF)" .strong(), "Save
reading copy" plain, "Start next Sunday" .strong(). Start's three
doors are indistinguishable in weight (4411, 4434, 4452). Fix: one
accent fill for the single primary action per step, used nowhere
else on that step (Print: "Open the print copy"; Start: "Use last
week's bulletin…"); bold text means one thing app-wide.

**M2. Both previews claim a fidelity they do not render.** Print
caption: "This is exactly what comes out of the printer"
(1812–1815); center preview: "Laid out with your real fonts, the
same lines the printer gets" (4822–4823). But preview text is
drawn with egui::FontId::proportional (the default UI font) at
4881, 5334, 2356; PreviewLine (964–971) carries only text/size/
x/y/w/align: no family, no bold, no italic. The PDF path honors
family/bold/italic (ob-pdf lib.rs:215–219). Line breaks and
widths are faithful (real shaping); letterforms, weight, and
slant are not. The sheet preview also draws panel outlines and
gray panel names (5306–5322) that never print, and the center
preview strokes every frame (4862–4873) while the PDF strokes
only Shape frames (ob-pdf 183–186). Fix: carry family + bold +
italic in PreviewLine and draw with the library families already
registered by refresh_fonts (1528–1546); drop or relabel guide
artifacts in draw_sheet; soften captions to what is true.

**M3. No product identity; first screen presents a fictional
church as the user's own.** main() (4906–4919) sets only the
window title; no with_icon, no .ico/.png asset in the repo
(verified by find); the Windows bundle stages no icon
(windows-bundle.yml:34–52): the taskbar icon is eframe's default.
First run: App::default() (985–1000) builds a template for
"Grace Lutheran Church"; profile buffers seed "123 Main Street"
(1006) while the document's address is empty, so the field
displays a fake address as data; tabbing through commits it
(lost_focus pattern, 4508–4515) and it can print on a real cover.
Fix: one icon wired with_icon + into the bundle; profile fields
genuinely empty with hint text, or an unmistakable "Sample
church" line in Start status until the name changes. (Converges
with the flow lane's first-run finding.)

**M4. No text-size accommodation; stock egui type scale.** No
set_style/text_styles/set_pixels_per_point anywhere. Hierarchy is
egui defaults; genuinely important small print (date-format help,
4582) renders as ui.small. Audience skews older; the only zoom in
the product is the Pages canvas. Fix: set text styles once at
startup (body ≈15–16px, small ≈12.5px); consider a "Bigger text"
toggle later.

## MINOR

- m1. Three different problem reds ((170,40,30) 4618/4660/4670/
  4811; (176,48,48) 3496; (150,60,50) 2267) plus amber
  (170,110,20) badges (3254, 3507). Define PROBLEM/WARN/GOOD/INFO
  constants once; darken the amber slightly.
- m2. Check results give problems and OKs identical weight:
  notes render as plain labels (4625–4628); only character
  findings get red + a fix button. Fix: "Check:" lines in the
  problem color with the fix-button pattern. (Converges with the
  flow lane's go-fix finding.)
- m3. One operation, several names (week advance: 4589 vs 4814);
  "New bulletin" (4281–4289) creates nothing; Fonts paths assign
  self.status directly instead of note()/fail() (3950–3959,
  4187), so font failures never get the error treatment.
- m4. Success feedback is small gray text (ui.small, 4819);
  saved paths embed verbatim (4744, 4770; save_flow 3710) and
  wrap to 3–4 tiny lines. Fix: body-size status; file name only
  in the message.
- m5. Window-pattern drift: Fonts window omits
  .collapsible(false) (3854–3858) and has no outer scroll (list
  only, 3859; sections 3851–4199 overflow laptop screens).
  Pages pager buttons are bare "<" / ">" (2066, 2075).
- m6. Two near-identical canvas blues: guides (0,150,200) 2636,
  selection (0,110,190) 2298. Unify on one canvas accent.
- m7. Fit-note wording narrates the roadmap ("…or we can shrink
  them in a later version", 1374–1377). Use the actionable
  sentence Check already has (4652).
- m8. Step-list markers (">" and "done -", 4360–4366) read as
  debug output. Bold current step plus "(done)" matches the
  house voice. (Converges with the flow lane's positional-marker
  finding.)
- m9. The guidance sentence renders as a plain label (4346), same
  weight as everything, while the top-bar heading slot repeats
  the app name. Make GUIDANCE strong or head the sidebar with it.

## Do not change

- The plain-words vocabulary on the guided path (Principle 3).
- The two-click armed labels on every discard path (4320–4325,
  4399–4403, 3300–3306, 3548–3554, 1727–1742).
- The review-window pattern (3223+, 3480+): counts first,
  tag+label+detail rows, same three buttons in the same order.
  The consistency model the rest should copy.
- The recovery window (1710–1716) as-is.
- Pages canvas furniture: inch rulers (2194–2248), guides with
  "The blue guides never print" (2113), right-click exact-inch
  entry (2830, 2765).
- Preview auto-fit (4836–4842) and the minimum window size 940×
  620 (4910): both from real user pain.
- Check runs itself on entering the step (1122–1124).
- Errors in red + strong as a pattern (4811–4816); it needs
  consistent plumbing (m3) and a reachable location (B1).

Lane's bottom line: fix B1 first; then M1+M2; M3/M4 are an
afternoon. The foundation (words, flow, guard patterns, review
windows) is the house style everything else converges to.
