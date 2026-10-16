The design is close in **workflow shape**, but not yet demonstrably close to “anyone can produce a correct bulletin unaided.” The five-step flow, Next Sunday, recovery, import review, and printer calibration remove substantial mechanical work (README.md, “The idea,” “Works today,” and “Imports and conversions”). The remaining risk is knowing **which bulletin to use, what still needs doing, whether carried-over content is still valid, and whether the copy being printed is the checked copy**. Those safeguards are not established by the supplied documentation. Nor is the unaided Sunday test established as passed: the pilot and print-identical editing spike remain unchecked (ROADMAP.md, “v0.1 — The Sunday Test”). My verdict: preserve the existing five steps; make the bulletin’s state and the next necessary action unmistakable inside them, rather than adding more tools.

## Reading this review

- **Evidence boundary:** This is a documentation review, not a hands-on test. “Not documented” means the supplied documents do not establish a safeguard—not that the code necessarily lacks it.
- **Payoff:** High / medium / low estimates concern weekly time and error reduction. Minute estimates below are hypotheses to measure in pilots, not measured results; overlapping savings must not be added together.
- **Proposal:** Every recommended change below is a proposal, not a claim about shipped behavior.
- **Documentation discrepancies:** README.md, “Not built yet,” says users must build from source; “Run it” describes a ready Windows bundle. ROADMAP.md lists some already-shipped imports again as future work. Scribus import is **later**, not shipped (ROADMAP.md, “Later, only if earned”). Publish one authoritative capability/release-status summary so reviewers and offices do not have to reconcile these accounts.

# 1. FRICTION AUDIT

The experienced admin’s path is chiefly a reuse problem; the pastor’s is a completeness problem; the stand-in’s is an orientation problem; the team’s is a shared-state problem. All four encounter the same five steps, but they cannot safely be expected to supply the same background knowledge.

| Weekly point and friction | Main persona; weekly payoff | Current-design evidence | Proposal |
|---|---|---|---|
| **Getting the app running.** A first-time substitute may encounter build instructions or an unfamiliar bundle before reaching Start. | **Stand-in; high** when covering, low in an already-equipped office | README.md, “Not built yet” and “Run it”; signed installers remain unchecked in ROADMAP.md, “v0.1.” | Give the office a tested launchable installation before cover is needed. Provide one plain download instruction per platform; do not expose build instructions as the office-user path. |
| **Start: identifying the right source.** The user must potentially distinguish last Sunday’s final file from a draft, an export, or an older copy. A designated weekly working file is not documented. | **Stand-in/team; high** | `.obd` open/save: README.md, “Works today”; desktop files are local: DEPLOYMENT.md, “Shapes,” item 1. | Start should show **“Working on: Sunday, 25 October”**, its status, and its predecessor. Prefer the designated working bulletin over a generic recent-files list. |
| **Start: advancing the correct week exactly once.** Next Sunday advances a week, but duplicate-week handling, service-date overrides, and protecting the source are not specified. | **Experienced admin/stand-in; high** | README.md, “The idea — Next Sunday”; ROADMAP.md, “v0.1,” Next Sunday checkbox. | Show the destination date before creation; reopen an existing working bulletin for that service instead of creating another. Preserve the previous bulletin unchanged. Allow **“Different service date…”** without adding a new mode. |
| **This Sunday: knowing what was cleared and what remains.** Clearing weekly slots does not itself tell a successor which inputs are still missing or awaiting confirmation. | **Pastor/stand-in; high** | README.md, “The idea — Fields” and “Next Sunday”; PRINCIPLES.md, §§4–5. | List required inputs in the existing step, with **“Needed,” “Entered,” “Needs checking.”** Show missing work directly, not merely an empty box in preview. |
| **This Sunday: entering the same fact twice.** Fields address this already, but the supplied docs do not establish whether every repeated weekly item is linked, especially after rescue import. | **Experienced admin; high** | README.md, “The idea — Fields”; imported words become editable boxes: “Imports and conversions.” | Audit repeated date, Sunday name, readings, hymn numbers, and sermon title. Enter each once and reuse it. During template setup, explicitly connect repeated appearances to the same field. |
| **This Sunday: trusting a plausible but wrong date or reading.** Check confirms the date is filled, not necessarily correct for this service; liturgical calendar data is planned. | **Pastor/stand-in; high** | README.md, “The idea — A Check step”; ROADMAP.md, “v0.5 — The Church Office.” | Check the intended service date against the bulletin date. Require a visible review of weekly facts. Later calendar packs should suggest values from the church’s chosen tradition, never silently select one. |
| **Announcements: deciding what survives next week.** A standing/weekly split is too coarse for notices that last three weeks or expire after an event. Announcement expiry behavior is not documented. | **Experienced admin; high** | README.md, “The idea — Next Sunday”; corpus classification: ROADMAP.md, “Import and conversion track — Template Analyzer.” | Offer **“This Sunday only”** by default and **“Keep through…”** when needed. Next Sunday removes expired notices and carries forward eligible ones. Retain an easy undo. |
| **Announcements: remembering what is outstanding.** A blank announcement section could mean “none,” “not received,” or “not entered.” | **Team/stand-in; high** | Announcements is a shipped step: README.md, “Works today”; no handoff states are specified in that section. | Distinguish **“Announcements still needed”** from **“No announcements this week.”** Allow one short pending note: “Waiting for food-bank collection dates.” |
| **Editing: fighting layout instead of finishing content.** Whole-box editing is shipped, while the shaped-caret/threaded-frame/undo spike is unfinished. The Pages window exposes manual positioning. | **Pastor/stand-in; medium** | ROADMAP.md, “v0.1,” text spike and desktop-shell notes; README.md, “Works today — A Pages window.” | Keep weekly edits in named inputs. Fit normal variation within agreed template limits underneath; never silently shrink text below the church’s readability minimum. Link exceptional overflow to the affected text, not to a layout lesson. |
| **Check: finding and fixing the reported problem.** Checks are documented, but direct navigation from a warning to its cause is not. | **Stand-in; high** | README.md, “The idea — A Check step.” | Every warning should say what is wrong, where it is, and offer **“Fix this.”** Example: “The sermon title does not fit on page 1. Shorten it.” |
| **Check: separating technical validity from editorial correctness.** Present checks cover date presence, order-of-service presence, overflow, fonts, and proximity to edges/folds—not verified readings or fresh announcements. | **Pastor/team; high** | README.md, “The idea — A Check step.” | Separate **“Print problems”** from **“This week’s details to confirm.”** Do not imply that clean geometry proves correct content. |
| **Check/Print: unsupported characters can disappear.** A typed cross or other unsupported character can print as a space. This is an actual documented correctness hazard. | **All, especially stand-in; high** | README.md, “Fonts today — PDFs include the real fonts.” | Detect every unsupported character before export. Block affected output until fixed, with a plain explanation and a safe replacement choice. Longer term, render it correctly; never quietly omit it. |
| **Print: choosing the right output and settings.** There are reading and print PDFs; native print dialogs are planned. A substitute may export the reading copy or use mismatched downstream settings. | **Stand-in/print volunteer; high** | README.md, “Works today — Make a PDF” and “The idea — Two-sided printing help”; ROADMAP.md, “v0.5,” native print dialogs. | Make **“Print this bulletin”** the main action. Keep **“Make a copy for email”** secondary. Until native printing lands, explain the next PDF-reader action and required paper, scaling, and two-sided settings. |
| **Print: knowing whether calibration still applies.** The app remembers a flip answer per printer, but invalidation after relevant setting changes is not documented. | **Print volunteer; medium** | README.md, “The idea — Two-sided printing help”; ROADMAP.md, “v0.1,” duplex wizard. | Remember the tested printer-and-setting combination. Ask for a test only when that combination is new or materially changed. Show a short summary of what was tested. |
| **After Check or Print: remembering which revision was approved or produced.** Autosave protects work but does not, by itself, establish approval or print status. | **Team; high** | README.md, “Works today — Autosave” and “Make a PDF”; DEPLOYMENT.md, “Accounts and roles,” file as truth. | Tie checks, confirmations, exports, and recorded printing to a document revision. Any content change invalidates the prior “ready” status. |
| **Recovery: choosing between restored and saved work.** Recovery is offered, but how the user identifies the newer or correct copy is unspecified. | **Stand-in/team; medium** | README.md, “Works today — Autosave”; ROADMAP.md, “v0.1,” autosave and recovery. | Say: **“We found newer work for Sunday, 25 October, saved at 4:12 pm.”** Offer “Continue this work” and “Keep the earlier copy”; preserve both until the choice is safe. |
| **Import/template setup: accepting a structurally plausible but semantically wrong draft.** Fidelity labels and human confirmation exist, but a stand-in may not know which text is standing or weekly. | **Experienced admin setting up cover; high initially, low weekly** | README.md, “Imports and conversions”; ROADMAP.md, “Import and conversion track — Template Analyzer.” | Treat template acceptance as office setup, not routine weekly work. Review recurring/weekly classifications and linked fields explicitly. Preserve the source for comparison. A stand-in should normally begin from an accepted `.obd`, not re-run rescue. |

**Important limit:** automation can carry text and detect inconsistencies; it cannot know that a pastor changed the sermon or that an announcement was cancelled without receiving that information. The interface must reveal those dependencies instead of pretending they have been solved.

# 2. STREAMLINING — RANKED BY WEEKLY PAYOFF

The ranking combines likely minutes saved with severity of prevented mistakes. It is deliberately not a ranking of engineering ease.

| Rank | Proposal: what changes | Main persona; payoff | Estimated saving and prevented error | Cost in user-facing complexity |
|---|---|---|---|---|
| **1** | **One unmistakable working bulletin per service.** Start locates it; Next Sunday creates it once; opening an existing week resumes it. | **Stand-in/team; high** | **3–8 minutes/week**, plus avoiding edits to last week or divergent drafts. | **Very low:** one dated card and one primary action. Engine manages identity and duplication. |
| **2** | **Visible weekly completion, attached to the actual content.** Missing inputs, explicit “none,” one pending note, and confirmations live in the existing steps. | **Pastor/team; high** | **5–10 minutes/week** of searching, asking, and rechecking; fewer omissions and stale facts. | **Low:** short status labels. No separate task board or manual percentage-complete tracker. |
| **3** | **Complete field reuse and date-bounded announcements.** Enter a fact once; recurring notices survive only as long as intended. | **Experienced admin; high** | **5–12 minutes/week**, depending on repeated content; fewer mismatched dates, hymn numbers, and expired notices. | **Low:** optional “Keep through…” is the only new recurring-content control. Template complexity stays underneath. |
| **4** | **Make Check actionable and honest.** Click-to-fix warnings, unsupported-character detection, and a distinct weekly-content confirmation. | **Stand-in/pastor; high** | **2–6 minutes/week**, plus potentially avoiding a full reprint. | **Low:** two short groups, not a technical preflight console. |
| **5** | **Make print readiness revision-specific.** Export the current checked revision, retain printer settings, and distinguish “PDF made” from “printed.” | **Print volunteer/team; high** | **2–5 minutes/week**, plus avoiding stale output and wrong-sheet printing. | **Low:** a dated print summary; details appear only when settings need attention. |
| **6** | **Resolve overflow where people write.** Show space trouble immediately and offer a route back to the affected wording. | **Experienced admin/pastor; medium** | **2–5 minutes in crowded weeks**; less end-stage layout repair. | **Low if bounded; high if turned into layout options.** Do not add fit algorithms for users to choose. |
| **7** | **Optional liturgical suggestions and spell check.** Implement the planned aids as suggestions with source/tradition visible. | **Pastor; medium** | **2–5 minutes/week** where applicable; fewer transcription errors. | **Medium:** traditions, exceptions, and dictionary choices create real burden. Defer until the basic session is reliable. |

Current foundations for these proposals: Next Sunday and fields (README.md, “The idea”); five steps, Check, export and recovery (README.md, “Works today”); whole-box editing and unfinished text spike (ROADMAP.md, “v0.1”); calendar packs and spell check (ROADMAP.md, “v0.5”).

# 3. HANDOFF AND COVER

**Main persona: team and stand-in. Weekly payoff: high.**

Desktop is local and login-free; later NAS deployment plans Owner/Editor/Viewer roles and one editor at a time—not simultaneous collaboration (DEPLOYMENT.md, “Shapes” and “Accounts and roles”). Therefore the immediate answer is **document-carried state**, not accounts, chat, or a collaboration system.

## Proposal: put the handoff inside the five steps

### Start: show what this file is and what happens next

Example:

> **Sunday, 25 October**  
> In progress · Saved today at 10:42 am  
> **Next: finish announcements**  
> Waiting for: food-bank collection dates  
> **Continue announcements**

Secondary information: “Started from Sunday, 18 October.” No need to show filenames prominently, though “Show file location” should remain available.

If no working bulletin exists:

> **Last bulletin: Sunday, 18 October**  
> **Start Sunday, 25 October**

If newer recovered work exists, resolve recovery before suggesting a new week.

### This Sunday and Announcements: distinguish missing from intentionally absent

Proposal states:

- **Needed** — no value supplied.
- **Entered** — supplied, but not confirmed where confirmation matters.
- **Confirmed for this Sunday** — someone checked the actual value.
- **None this week** — an explicit answer, not an empty box.
- **Waiting for…** — optional short note attached to the relevant item.

Do not ask people to maintain these states separately from the work. Entering a value changes its state automatically; changing a confirmed value removes its confirmation.

Use human labels:

> Sermon title — Waiting for Pastor Lee  
> Announcements — No announcements this week  
> Readings — Entered; please confirm

### Check: show the whole handoff without another screen

Proposal summary:

> **Two things left**  
> Confirm the readings.  
> Add the food-bank dates, or mark that announcement “Not this week.”

After those are resolved:

> **Ready to make the print copy**  
> Checked at 11:18 am.

“Ready” must mean the documented technical checks and the applicable weekly confirmations have passed for this revision. It is not a guarantee that all information received was true.

### Print: distinguish intentions from events

Proposal states:

- **Needs checking**
- **Ready to make print copy**
- **Print copy made at 11:20 am**
- **Printing recorded at 11:35 am**

Do not label a PDF export “Printed.” Without reliable printer completion feedback, ask **“Did the copies print correctly?”** and record the answer.

After a subsequent edit:

> **Changed since the print copy was made. Check again before printing.**

## Three concrete handoffs

1. **Successor takes over midweek:** opens the working bulletin, sees the dated status and unresolved item, continues directly there. The predecessor’s memory is no longer the status database.
2. **Two people divide work on desktop:** they work sequentially in the same file. One enters announcements and leaves any pending note; the next continues This Sunday. For off-machine collection, use a simple copyable list of requested inputs—not competing editable bulletin copies.
3. **Stand-in arrives on Sunday morning:** Start says either “Ready to make print copy” or precisely what remains. No hunt through pages to discover whether blank content is intentional.

## Later NAS proposal: protect the single-editor rule

When someone is editing:

> **Alex is editing this bulletin. You can view it now.**

Allow viewing while preventing another editor from overwriting work. A stale-session recovery must preserve saved work before allowing takeover; a quiet last-save-wins conflict is unacceptable.

Viewer can print a ready revision, consistent with the planned Viewer role. If the revision needs fixing, say **“This bulletin needs changes before printing. Ask an editor to finish it.”** Editing controls must not become a surprise permission trap halfway through the job.

Do not put account identities or permissions in `.obd`; the supplied deployment design explicitly reserves accounts for the server (DEPLOYMENT.md, “Accounts and roles”).

# 4. THE STAND-IN TEST

**Main persona: first-time volunteer. Weekly payoff: high during cover; medium as a routine reliability test.**

This is a **scenario walkthrough, not observed usability evidence**. Assume the app is already installed, last Sunday’s accepted `.obd` is available, and this week’s source material exists. Missing source material cannot be manufactured by the program.

The timings are checkpoints for a roughly 25-minute session, excluding a long print run. Each remedy is a **proposal**.

| Minute | Where the volunteer can stall, guess, or produce a wrong result | What the program should do |
|---:|---|---|
| **0** | Cannot identify or launch the app. Distribution guidance conflicts. | Provide a plainly named launchable app and a tested office installation. *(README.md, “Not built yet,” “Run it.”)* |
| **1** | Start offers choices, but the volunteer does not know which file is final. | Display the designated last bulletin and current working bulletin by service date. *(Five-step Start: README.md, “Works today”; file choice safeguard not documented.)* |
| **2** | Opens last Sunday and starts overwriting it. | Make **“Start this Sunday”** prominent; preserve the predecessor. *(Next Sunday: README.md, “The idea.”)* |
| **3** | Runs Next Sunday twice or gets the wrong destination date. | Show the destination; resume an existing week; offer “Different service date…” *(README.md, “The idea — Next Sunday.”)* |
| **4** | Sees cleared content but cannot tell what must be filled. | Show a short “Needed this week” list inside This Sunday. *(Clearing weekly content: README.md, “The idea.”)* |
| **5** | Confuses the date with the church’s Sunday name. | Use separate plain labels and show both together in preview. Do not infer a tradition silently. *(Fields: README.md, “The idea”; calendar packs planned: ROADMAP.md, “v0.5.”)* |
| **6** | Cannot find the authoritative readings. | Show a missing-input prompt and any saved handoff note; do not treat old readings as a fallback. *(Weekly clearing: README.md, “The idea — Next Sunday.”)* |
| **7** | Enters a reading reference in one place but misses another. | Enter once and update every linked occurrence. *(Field mechanism: README.md, “The idea — Fields”; full mapping coverage not documented.)* |
| **8** | Types hymn dots and spacing manually. | Provide title/number inputs and let the engine create leaders. *(README.md, “The idea — Dot leaders.”)* |
| **9** | Mistypes a hymn number or copies last week’s sermon. | Show the weekly facts together for confirmation; flag unchanged weekly content as a question, not an automatic error. *(Documented Check scope: README.md, “The idea — A Check step.”)* |
| **10** | Pastes a long sermon title and does not notice it will not fit. | Warn beside that input immediately; link back from Check. *(Measured overflow: README.md, “The idea — A Check step.”)* |
| **11** | Enters Pages and becomes distracted by box positions and guides. | Keep the weekly path in named inputs; expose page arrangement only when deliberately requested. *(README.md, “Works today — A Pages window”; PRINCIPLES.md, §§2–3, 9.)* |
| **12** | Carries forward an expired notice or deletes a still-active one. | Apply saved “Keep through…” dates and show what expired, with undo. *(Carry/clear behavior: README.md, “The idea — Next Sunday”; expiry not documented.)* |
| **13** | Leaves Announcements empty, assuming that means none. | Ask **“No announcements this week?”** or retain “Announcements still needed.” *(Announcements step: README.md, “Works today.”)* |
| **14** | Types a cross or another unsupported character. | Flag it where entered; block affected export rather than printing a space. *(README.md, “Fonts today.”)* |
| **15** | Moves to Check but cannot interpret or locate warnings. | Each warning offers “Fix this” and identifies the page and wording. *(Plain-language checks: README.md, “The idea — A Check step.”)* |
| **16** | Believes a clean Check proves the readings and date are correct. | Clearly separate print checks from “Confirm this week’s details.” *(Current check list: README.md, “The idea — A Check step.”)* |
| **17** | Skips visual inspection because the app said “ready.” | Show a concise page preview review focused on cover, service order, and announcements; do not demand a designer’s inspection. *(Preview: README.md, “Works today.”)* |
| **18** | Chooses the reading PDF to print folded copies. | Primary action: “Make print copy”; secondary: “Make copy for email.” *(README.md, “Works today — Make a PDF.”)* |
| **19** | Uses the wrong printer, paper size, scaling, or flip setting. | Show the remembered tested combination and plain settings; offer test-sheet help when needed. *(Duplex help: README.md, “The idea”; native print dialogs planned: ROADMAP.md, “v0.5.”)* |
| **20** | The test sheet fails, and the volunteer guesses a technical flip setting. | Continue the existing plain-words questions using pictures; remember the corrected answer. *(ROADMAP.md, “v0.1 — Duplex test-sheet wizard.”)* |
| **21** | Prints a full run before discovering wrong orientation or folding. | For a new combination, lead with one folded sample. Keep the full run secondary until that sample is confirmed. *(Existing test-sheet foundation: README.md, “The idea — Two-sided printing help.”)* |
| **22** | Corrects a typo but prints the previously exported PDF. | Mark the old print copy out of date and generate a fresh checked revision. *(Export exists: README.md, “Works today”; revision linkage not documented.)* |
| **23** | Cannot tell whether the app saved, exported, or actually printed. | Show separate saved/exported/printing-recorded facts. *(Autosave and export: README.md, “Works today.”)* |
| **24** | Finishes but leaves no understandable state for Monday’s admin. | Record the printed revision and any unresolved note; show “Start next Sunday” without silently creating another bulletin. *(PRINCIPLES.md, §4; Next Sunday: README.md, “The idea.”)* |

**Pass criterion — proposal:** an unfamiliar volunteer produces the correct date, supplied weekly content, complete readable pages, and correctly folded output; identifies missing information rather than inventing it; and leaves an intelligible saved state. Record wrong outputs and assistance requests, not just completion time. This strengthens the existing unaided Sunday acceptance test rather than replacing it (ROADMAP.md, “v0.1 — The Sunday Test”).

# 5. SCOPE DISCIPLINE — WHAT NOT TO BUILD

These are proposed exclusions or deferrals for the streamlining goal. The governing test is strict: one-sentence usefulness, plain words, recurring automation, and conformity to office habits (PRINCIPLES.md, §§1–5, 9).

| Do not build for this goal | Persona protected; weekly payoff | Reason and simpler substitute |
|---|---|---|
| **A project-management layer:** boards, assignments, due dates, dependencies, approval chains. | **Stand-in/team; high** | It creates a second thing to maintain. Use content-derived states and one attached pending note. |
| **Simultaneous editing, comments, presence feeds, or bulletin-copy merging.** | **Team/stand-in; high** | Conflicts are harder than sequential handoff. Preserve one editor at a time. This also respects DEPLOYMENT.md, “Accounts and roles,” and README.md, “What we will not build.” |
| **Desktop logins or role selection before work begins.** | **Stand-in; high** | Permission administration solves no single-machine weekly problem. Desktop is explicitly login-free (DEPLOYMENT.md, “Shapes” and “Accounts and roles”). |
| **Expert/beginner modes or mandatory shortcut-profile selection.** | **Stand-in; medium** | Different paths make cover harder. Keep one main workflow; planned keymaps may remain optional in Keys, never onboarding homework (README.md, “The idea — Planned: familiar keys”). |
| **A rule language for recurrence, styling, or announcement scheduling.** | **Experienced admin/stand-in; high** | It makes the human program the app. Use dates and ordinary named fields. GREP-style rules are already excluded (README.md, “What we will not build”). |
| **Automatic shrinking until everything fits.** | **Pastor/stand-in; high** | “Fits” is not “readable.” Respect a template minimum; surface the exceptional text that needs shortening. This follows print reliability and surface simplicity (PRINCIPLES.md, §§2, 6). |
| **Unreviewed liturgical or AI-generated content.** | **Pastor/stand-in; high** | Plausible content can be confidently wrong. Use sourced suggestions and human confirmation; AI generation is already excluded (README.md, “What we will not build”). |
| **Weekly template relearning or font exploration on the main path.** | **Stand-in; medium** | Setup should not recur every Sunday. Keep accepted templates stable and Font Library controls optional (README.md, “Imports and conversions” and “Fonts today”). |
| **Perfect native Publisher reading, full PDF editing, or extra import formats as the next streamlining investment.** | **Experienced admin; low weekly** | Rescue is valuable migration work, not the recurring weekly bottleneck. Preserve honest draft review; native `.pub` and `.indd` reading are excluded, Scribus is later (README.md, “Imports and conversions”; ROADMAP.md, “Later, only if earned”). |
| **NAS deployment as a prerequisite for good handoff.** | **Team; high** | Solve understandable document state now. NAS, accounts, and network administration must remain later and optional (DEPLOYMENT.md, “Shapes” and “Accounts and roles”). |

# The five changes I would make first

1. **Proposal — make Start identify and resume one dated working bulletin, with duplicate-safe Next Sunday.**  
   **Stand-in/team; high payoff.**

2. **Proposal — attach missing/entered/confirmed/none states and a short pending note to the existing weekly inputs.**  
   **Pastor/team; high payoff.**

3. **Proposal — finish enter-once field reuse and add “This Sunday only / Keep through…” announcements.**  
   **Experienced admin; high payoff.**

4. **Proposal — make Check click-to-fix, detect unsupported characters, and distinguish technical checks from weekly-fact confirmation.**  
   **Stand-in/pastor; high payoff.**

5. **Proposal — tie readiness and print output to the current revision, with tested printer settings and an honest “PDF made” versus “printed” distinction.**  
   **Print volunteer/team; high payoff.**

These five changes make the existing workflow trustworthy. They should precede new modes, broader publishing capabilities, or collaboration infrastructure.