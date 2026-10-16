# OpenBulletin design review

## Basis of judgment

The packet supports a real weekly-production app, not merely a concept. The guided shell, page manipulation, booklet geometry, weekly rollover, checks, and recovery are described as shipped.

It does **not** establish that the following are shipped:

- The Template Analyzer, mixed-format learning, or seasonal clustering.
- The **Matched / Check this / Picture only** import labels and **Accept / Edit / Cancel** draft workflow.
- Consistent work-state and next-action feedback throughout the five steps.
- Undo coverage beyond the explicitly described “Next Sunday” undo.

Those remain requirements or unverified claims for this review. Tests and one Windows run establish more than a mockup, but do not establish that either persona can finish unaided.

## 1. Verdicts and real-week walkthroughs

### Office administrator: likely successful on an established bulletin; unaided first-time rescue is not established

The philosophy holds best on the administrator’s familiar weekly route. It is weakest where a predecessor’s document must become a trustworthy starting point.

| Moment | What holds | Where the philosophy falls short or remains unproven |
|---|---|---|
| **First open ever** | One open door fits the pile of Publisher files and PDFs. Church profile fields reduce repeated typing. Two-click protection makes a fresh start safer. **P3, P6, P7.** | Opening a file is not the same as rescuing it. The shipped description does not establish how editable content, substitutions, and picture-only content are explained or accepted. The analyzer cannot be counted on. **P7**, and consequently **P4**. |
| **Thursday: open last week** | Remembering the last bulletin and providing one “Next Sunday” command closely match the actual job. Carrying standing content, clearing weekly content, honoring expiry dates, preserving the current file, and allowing undo are strong fits. **P1, P5, P6.** | The packet does not describe a visible explanation of what rollover changed. A busy administrator needs to see the new date and understand what cleared without inspecting every panel. This is a **P4/P5 feedback gap**, not evidence that rollover is incorrect. |
| **Replace weekly material** | Fields fill recurring locations. Direct dragging, inch readouts, snapping, fractions, and right-click placement respect office habits. **P2, P3, P9.** | Clicking text does not put the caret in the text. Editing happens in another text area. That is a genuine, acknowledged **P9 debt**: the paper object stops behaving like the thing being edited. |
| **Check and print** | Plain-word findings with jumps to named boxes are unusually well matched to interruption-heavy work. Printer-sheet previews and remembered two-sided questions support booklet printing. **P2, P4, P6, P9.** | “Open the print copy” ends at opening a file, not at producing sheets. The packet does not establish guidance across that handoff or how serious findings are resolved. A stale-copy warning is good, but its prominence and recovery action are unspecified. **P4/P6.** |
| **Prepare for vacation** | Local files and a familiar five-step route avoid account setup and a second workflow. **P1, P3.** | “Last bulletin remembered” is personal continuity, not necessarily a handoff. No shipped handoff state is described that tells the pastor which Sunday this is, what remains, and whether the print copy is current. **P4/P6.** |

**Verdict:** With a usable existing bulletin and a familiar printer, this administrator has a credible route to success. For a new administrator inheriting unfamiliar files, the packet does not yet justify “unaided today.”

### Pastor: plausible for word changes and proofing; unaided vacation production is not established

The pastor needs the app to explain itself again after months away. The same features that save the administrator time must make their consequences legible to the pastor.

| Moment | What holds | Where the philosophy falls short or remains unproven |
|---|---|---|
| **First encounter** | The five steps provide a recognizable route. Plain words and liturgical fields fit someone who thinks in content, not layout. **P1–P4.** | If handed an imported draft, the pastor needs to know whether it is a faithful rescue or merely a visual approximation. That shipped explanation is not established. **P7.** |
| **Normal week: title, readings, proof** | Field filling can change repeated content without requiring layout work. Checks can identify a problem and take the pastor directly to it. **P2–P4.** | The packet does not describe how an intermittent user identifies which field feeds which printed locations. Whole-box editing also imposes a less familiar detour when a correction is made on the page. **P4/P9.** |
| **Print morning, perhaps another printer** | Questions in plain words, remembered per printer, are better than assuming one universal duplex setting. Sheet previews address the actual paper object. **P3, P6, P9.** | Remembered answers help only when they are still appropriate. The packet does not establish a first-use confidence check for an unfamiliar printer, or guidance after the print copy opens. Booklet orientation mistakes can waste the entire run. **P4/P6.** |
| **Vacation: run the whole week alone** | One rollover command and one guided route reduce the amount to relearn. Explicit user initiation, preserved prior files, and rollover undo support control and forgiveness. **P1, P5, P6.** | The pastor must establish the right starting file, verify rollover, replace announcements, resolve checks, and print. A five-step shell alone does not establish visible answers to those questions. **P4** is the decisive unverified requirement. |

**Verdict:** The pastor could plausibly make bounded content changes. The packet does not yet demonstrate unaided vacation production, particularly at an unfamiliar printer. That is not proof of failure; it is the highest-risk acceptance test still missing.

## 2. Friction findings, ranked by Sunday-deadline damage

“Unverified” below means the packet does not demonstrate the required behavior, not that the feature is necessarily absent.

### 1. The final print handoff is not demonstrated end to end
**Personas:** Both; especially the pastor.  
**Principles:** P4 guidance, P6 printer predictability.

- **Moment:** On Print, the person sees the sheet preview and chooses **“Open the print copy.”**
- **Expectation:** The next action will lead reliably to the intended folded booklet.
- **Described behavior:** A print copy opens. The packet does not describe the remaining print interaction.
- **Damage:** Wrong scaling, sides, or orientation can invalidate the entire run when there is no recovery time.
- **Finding:** A downstream interaction gap in the packet, not evidence of incorrect imposition. Delegating printing is acceptable; leaving its next step unexplained is not.

### 2. A returning user cannot be shown to establish “this is the right Sunday”
**Personas:** Pastor on vacation cover; interrupted administrator.  
**Principles:** P4 feedback, P5 visible automation, P6 stability.

- **Moment:** Start offers the remembered bulletin; the user invokes Next Sunday.
- **Expectation:** Clear confirmation of the target Sunday and what was carried, cleared, or retained.
- **Described behavior:** Correct rollover rules are implemented, but their on-screen explanation is not described.
- **Damage:** A technically successful rollover can still leave unnoticed stale announcements or an incorrect starting week.
- **Finding:** Automation is shipped; understandable verification is unverified.

### 3. An inherited file may look usable before its limitations are understood
**Personas:** New administrator; pastor forced to use a predecessor’s file.  
**Principles:** P7 honest rescue, P2 faithful appearance.

- **Moment:** The user opens a PDF or Publisher bulletin through the common door.
- **Expectation:** “I can update this for Sunday.”
- **Described behavior:** Opening formats and font matching are shipped. Honest import labels and draft acceptance are not established.
- **Damage:** Discovering late that content is picture-only or needs substantial checking can destroy the preparation window.
- **Finding:** One door satisfies access, not the entire rescue principle. Font matching is not evidence of faithful, editable import.

### 4. Finding a print problem is easier than knowing it is resolved
**Personas:** Both; particularly an untrained substitute.  
**Principles:** P4 next step, P6 no printer surprises.

- **Moment:** Check reports overflow, a missing font, or an unprintable character and jumps to the box.
- **Expectation:** “Tell me what to change and how I know the problem is gone.”
- **Described behavior:** Plain findings and navigation are shipped; repair guidance and resolution feedback are unspecified.
- **Damage:** The person can reach the problem yet still print it unresolved, or become stuck attempting a layout repair.
- **Finding:** Good diagnosis; the packet does not establish closure. Do not assume every finding should block printing—some may be acceptable with informed judgment.

### 5. Text correction breaks the paper-object interaction
**Personas:** Both; worse for the intermittent pastor.  
**Principle:** P9 direct manipulation.

- **Moment:** The person clicks a sermon title or announcement on the page.
- **Expectation:** A caret appears there.
- **Actual behavior:** They edit the whole box in a separate text area.
- **Damage:** Adds a translation step to ordinary proofreading and makes precise corrections less direct.
- **Finding:** A demonstrated betrayal of the principle, openly recorded as debt. The shipped move/resize interaction otherwise honors P9 well.

### 6. Naming is not consistently noun first
**Personas:** Both.  
**Principles:** P3 action naming; P1 consistency.

**“Open the print copy”** is verb first. **Start, Check, Print** also do not follow the literal noun-first rule, although step names are not necessarily action buttons.

This is a real law/implementation mismatch, but a low-damage one. Do not spend Sunday-safety effort renaming familiar words merely to satisfy syntax. Either consistently apply the rule to actions or clarify its scope in the principles.

## 3. Where the personas diverge—and how one app can serve both

| Principle | Administrator’s need | Pastor’s need | One-app response |
|---|---|---|---|
| **P4: Guidance** | Resume quickly after interruptions without repeating instruction. | Reconstruct the workflow after months away. | Keep the same five steps; show a compact status and one next action, with optional explanations at the point of use. |
| **P5: Automation** | Fast rollover with little ceremony. | Understand what rollover did before trusting it. | Give both the same concise change summary. Do not add a separate beginner wizard or silently perform more actions. |
| **P6: Forgiveness** | Recover recent edits and interruption-related mistakes. | Recover a mistaken starting file or misunderstood command. | Make the current file/week unmistakable and consequential actions explicitly reversible where supported. Show recovery context, not merely a recovery offer. |
| **P6: Printing** | Reuse a known printer setup. | Validate a different printer with little remembered knowledge. | Preserve per-printer answers, show which setup is active, and offer an explicit test-sheet route when needed. |
| **P9: Manipulation** | Efficient corrections, familiar shortcuts, exact placement. | Edit words without learning box geometry. | Make on-page text editing the main path; retain exact numbers, right-click tools, and shortcut presets as optional power. |

Frequency should change how much help is expanded, not the underlying workflow or safety standard.

## 4. The five changes I would make first

### 1. Close the print-copy-to-paper loop
**Interaction change:** Before opening the copy, show the active printer assumptions and whether the copy is current. State what to do in the print dialog. Provide a deliberate test-sheet route for unfamiliar printer setups, and a clear regenerate action for a stale copy.

**Restores:** **P4 and P6.** Keeps the human responsible for firing the print job while reducing last-minute uncertainty.

### 2. Make weekly state and rollover visible
**Interaction change:** Show the bulletin’s Sunday, file identity, and remaining work throughout the flow. After Next Sunday, summarize the date change, carried standing content, cleared weekly content, and retained dated announcements; expose the existing undo.

**Restores:** **P4, P5, P6.** Helps both interruption recovery and vacation handoff without a second mode.

### 3. Complete the honest import contract before promising template learning
**Interaction change:** Make imported files explicit drafts with **Matched / Check this / Picture only** explanations attached to the affected content, followed by **Accept / Edit / Cancel**. Explain what acceptance means; it must not imply that all print risks are resolved.

**Restores:** **P7, supported by P2 and P4.** Prioritize trustworthy rescue over the unverified analyzer.

### 4. Turn Check findings into completed repairs
**Interaction change:** Keep the existing jump-to-box button, add a short suggested next action, and visibly update the finding after a repair. Distinguish issues that threaten output from cautions the person can knowingly accept.

**Restores:** **P4 and P6.** Navigation becomes a route to completion rather than merely a route to the defect.

### 5. Add in-place text editing
**Interaction change:** Clicking text places a caret on the page. Keep whole-box editing available as an alternative and preserve move/resize controls.

**Restores:** **P9.** Removes a recurring mismatch between what the user grabs and what they must operate.

## 5. Questions for the pastor interview

Ask for recent examples and actual bulletins, not just preferences.

### The weekly reality
1. Walk us through the last bulletin you personally produced. What happened outside the publishing app?
2. What changes every week, what usually stays, and what looks permanent but occasionally must change?
3. Who supplies each item, and which items routinely arrive late?
4. Tell us about a bulletin error the congregation noticed. When could it have been caught?

### Vacation cover and interruption
5. The last time the administrator was away, how did you identify the correct starting file and unfinished work?
6. After months away from an app, what do you forget first?
7. What would you need to see on the first screen to know, “This is this Sunday’s bulletin, and here is what remains”?
8. When interrupted midway through preparation, how do you reconstruct where you stopped?

### Liturgical and seasonal judgment
9. When do date, Sunday name, readings, or service order depart from the usual pattern?
10. How do Christmas, Easter, funerals, and combined services change the paper itself—not just the words?
11. How do you decide when an announcement expires? What exceptions defeat a simple “keep through” date?
12. Which mistakes would embarrass you, and which would make you stop printing?

### Printing and responsibility
13. Show us how the 16-page booklet is printed, checked, folded, and assembled. Where do errors usually occur?
14. What changes when you print a proof somewhere else? What has gone wrong with sides, orientation, or scale?
15. Who gives final approval, and how do you distinguish the approved version from a later edit?
16. Under a Sunday deadline, would you rather correct a flagged problem, accept it knowingly, or use another route? Give a real example.

### Test the philosophy against behavior
17. With an old Publisher file or PDF, what would you assume “opened successfully” means?
18. What wording would tell you that some content is only a picture and cannot be updated as text?
19. If you click words on the page, what do you expect to happen?
20. Give you an unfamiliar bulletin and printer: what information would let you proceed confidently, and what would make you call the administrator?

The most valuable follow-up is an observed vacation-cover rehearsal with a real inherited file and the real booklet printer. That tests unaided success; the document packet cannot.