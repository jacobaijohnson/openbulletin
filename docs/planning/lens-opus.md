# OpenBulletin design review: the two personas through PRINCIPLES.md

## 0. What this packet can and cannot support

The shipped list is concrete and mostly credible. Several claims in the principles are **not** in the shipped list, so I treat them as aspiration until shown otherwise:

| Claim in principles | Status in shipped list |
|---|---|
| Template Analyzer (learns layout from 3+ old bulletins, Accept/Edit/Cancel) | **Absent.** Not listed as shipped. |
| Seasonal variants clustering separately | **Absent.** |
| Imports land as drafts with Matched / Check this / Picture only labels | **Not confirmed.** Shipped list says "one open door for every format" and font matching for imports. It does not mention the draft or the labels. |
| "Never surprise at the printer" | **Partial.** The app's job ends at "Open the print copy." The actual printing happens elsewhere. |
| "Nothing moves or vanishes between visits" | **In tension** with "weekly content clears." No report of what cleared is described. |
| "The app states the work's state and what happens next" | **Not described** for Start, Next Sunday, or returning after an interruption. |
| "Every screen passes an explain-it-simply test" | **No evidence.** "Run once by a human on Windows" is a smoke test, not a user test. Neither persona has used this app. |

Everything below judges the design as described, not observed behavior.

---

## 1. Verdict per persona

### Office administrator: probably yes for the weekly cycle; unproven for the first week; exposed on print morning

**Where the philosophy holds:**
- **One open door (P7).** It matches how they think: "open the old one."
- **Five steps with one primary action each (P4).** This fits a person who gets interrupted every ten minutes.
- **Next Sunday (P5, P6).** It is exactly their well-worn path made into one command, and it is non-destructive with Undo.
- **Check findings with jump-to-box buttons.** This is See-and-Point done right. The admin never hunts for the problem.
- **Power in familiar shapes (P9).** Inch rulers, fractions, right-click menus, and keybinding presets that only set defaults all honor the Publisher habits they already have.
- **Freshness warning.** It is the right guard for "I fixed the typo after I made the PDF."

**Where it breaks:**
- **First open ever.** Their raw material is a pile of Publisher files and PDFs. The packet does not say whether an imported bulletin's date, readings, and hymns become *fields*. If they don't, Next Sunday has nothing to advance, and the admin is back to hand-editing every week. The bridge from "old file" to "fields that automate" is the Template Analyzer, which is not shipped. This is the single most important thing to verify.
- **Print morning.** The app produces a correct print copy, then hands the most dangerous step to someone else's print dialog: scaling, short-edge flip, page range. See finding F3.
- **Habit trap.** Their habit is "open last week's and change it." "Pick up where you left off" invites exactly that, which bypasses Next Sunday. See finding F5.

### Pastor (vacation handoff): not reliably unaided today

He is the infrequent user, the accountable one, and the one most likely to be on a different machine and printer. The design has the right parts, but the shipped app does not protect him at the moments that matter:

- **He may start from the wrong file and date.** "Pick up where you left off" remembers *the last bulletin on that machine*. On his laptop, that could be one from months ago. Next Sunday would then advance from the wrong base.
- **He may not find the current files at all.** The app is all local, with no cloud. If the current bulletin lives on the office computer, the packet offers him no path to it, and gives no hint of one.
- **He doesn't know what the automation did.** Which announcements carried? Which expired? Which fields cleared? None of this is reported.
- **Check won't catch his worst mistakes.** It looks for overflow, folds, fonts, and unprintable characters. It does not look for last week's readings, a blank hymn number, or a Sunday name that didn't change. Those are exactly the errors the congregation will blame him for.
- **A new printer asks questions he can't answer.** Two-sided answers are remembered per printer, so his printer starts blank, and he does not know whether the booklet flips on the short edge.

---

## 2. Friction findings, ranked by damage on a real Sunday deadline

**F1. Wrong source, wrong date.** *P4 Feedback, P5, P6. Hits the pastor (hard) and the admin (occasionally).*
- **On screen:** Start offers "pick up where you left off," which is the last bulletin opened on this machine.
- **Pastor expects:** "this week's bulletin."
- **What happens:** On his machine, that is a bulletin from months ago. He runs Next Sunday. The packet says it "advances the date," but not whether it computes the coming Sunday from today or adds seven days to the file's date. If it adds seven days, the cover prints the wrong date. Nothing on screen says "you are making the bulletin for Sunday ___ from the one dated ___."
- **Damage:** A wrong date on the cover is the most visible, most public error a bulletin can have.

**F2. Check has no findings about the content that changes weekly.** *P6 never surprise, P5. Hits both, most of all the pastor.*
- **What happens:** Next Sunday clears weekly content. If a hymn number or reading is left blank, or never cleared and still shows last week's, Check passes it.
- **Admin expects:** That "Check" means the bulletin is right. The Check step is labeled as the safety net, so it gets trusted beyond what it checks.
- **Missing findings:**
  - field left empty
  - weekly field identical to last week
  - Sunday name unchanged while the date changed
- **Damage:** Wrong readings or hymn numbers mean the worship leader announces a hymn that isn't what's printed, with 200 people watching.

**F3. The print handoff leaves the app.** *P6 never surprise at the printer, P9 conform to the worker. Hits both, worst for the pastor at an unfamiliar printer.*
- **On screen:** "Open the print copy." The copy opens in whatever the machine uses for PDFs.
- **What happens:** That program's print dialog is where the booklet is won or lost:
  - "Fit" versus "Actual size" scaling
  - long-edge versus short-edge flip
  - all pages versus current page
- **Why the app's preparation doesn't save it:** The app asked good two-sided questions and remembered the answers, but those answers only shape the file. The dialog that actually prints knows nothing about them.
- **Damage:** A 16-page saddle booklet printed long-edge flip has upside-down backs. You discover that after the whole run, on Saturday night.

**F4. Next Sunday clears and expires things silently.** *P6 Perceived Stability, P4 Feedback. Hits both.*
- **What happens:** "Weekly content clears" and "keep through" dates expire. Both make content vanish between visits, which is exactly what P6 forbids. Undo exists, but you can only undo what you noticed.
- **Example:** An announcement with a wrong keep-through date, such as a funeral or a pledge deadline, disappears. No one sees it go.
- **Damage:** A missing announcement the pastor promised from the pulpit.

**F5. Editing last week's file instead of making next week's.** *P4 next step visible, P5. Hits the admin's habit and the new admin.*
- **What happens:** The admin's ingrained path is "open last week's, change the date." The app's correct path is "Next Sunday." Start presents "pick up where you left off" and the open door. The packet does not say Start ever proposes Next Sunday, even when the last bulletin already has a print copy, which means it's done.
- **Consequences:**
  - Autosave makes the overwrite permanent, so last week's bulletin is gone as an archive.
  - Next Sunday's carry logic never ran, so keep-through dates were never honored.
- **Damage:** Medium on the day, compounding over weeks.

**F6. Imported bulletins may not automate.** *P7, P5. Hits the new admin in week one.*
- **What happens:** The admin opens the predecessor's .pub. Whether it lands as an accept-or-cancel draft with honest labels is not confirmed as shipped.
- **The real gap:** Even if it opens perfectly, nothing in the packet turns "Oct 11, 2026" printed on the imported cover into the *date field*.
- **Damage:** The automation promise (P5) silently doesn't apply. The admin concludes the app is "just a worse Publisher." That causes abandonment, not a bad Sunday, but it is the most likely way this product loses an office.

**F7. Text is edited away from the page.** *P9 Direct Manipulation, P6 WYSIWYG. Hits both; it is the pastor's main task.*
- **What happens:** The pastor proofs on paper and circles "line 3 of the prayer of confession." The admin, or the pastor himself, clicks the box and edits in a separate text area. The page is not the edit surface.
- **Unknown:** The packet does not say whether the page updates as you type. If it doesn't, overflow is only discovered at Check.
- **Status:** This is a recorded debt (in-place caret editing). For the pastor's main task, correcting words, it is the most-used gap in the app.

**F8. The unprintable character is shown on screen but won't print.** *P6 WYSIWYG. Hits both.*
- **What happens:** The Latin cross displays on screen, and Check correctly flags it. But the screen showed it, which is a WYSIWYG break that Check papers over. The fix is to jump to the box, then find one character inside a whole-box text area.
- **Damage:** Modest, but it teaches distrust of the screen.

**F9. Layout jargon on the main path.** *P3. Hits both, worst for the pastor.*
- **The words:**
  - "story" (an InDesign word) in "per-story keep through dates"
  - "panel"
  - "spreads"
  - "guides" and "snapping"
  - "print copy"
  - "printer encoding"
  - "fold presets" and "saddle booklet"
  - "metric twins"
- **What to do:** Some belong only in the canvas window, which is fine. "Story" and "encoding" leak into the weekly path and should go. The admin says "announcement." Nobody says "encoding."

**F10. The noun-then-verb rule is ambiguous, and the shipped labels break it.** *P3. Low damage, but cheap.*
- **The ambiguity:** Apple's Explicit and Implied Actions is about *interaction order*: select the thing, then act. Right-click on a box satisfies that. PRINCIPLES.md reads like *label grammar*.
- **Under the label reading:** "Open the print copy" and "Pick up where you left off" violate it.
- **What to do:** Decide which rule you mean and write it down, or the team will argue it label by label.

**F11. Returning after an interruption.** *P4 Feedback. Hits the admin daily.*
- **What happens:** The phone rings mid-Announcements. Does the app reopen on the step they were in, with the state shown ("Announcements: 3 of 5 replaced")? The packet doesn't say. Autosave protects the work, not the person's place.

**F12. Fonts and settings don't travel with the file.** *P6, P9. Hits the pastor only.*
- **What happens:** On a second machine:
  - the missing-fonts finding fires
  - printer answers are absent
  - church profile fields may be absent (the packet doesn't say where they live)
- **Assessment:** Check catches fonts. The rest just looks "different" to him with no explanation.

---

## 3. Where the personas diverge under the same principle

| Principle | Administrator (weekly, fluent) | Pastor (every few months, accountable) |
|---|---|---|
| **Guidance (P4)** | Learns the five steps in two weeks. Wants to jump straight to Announcements. Guidance that forces order becomes a trap. | Needs every step and needs to be told which steps the automation already did. Has no memory of the flow. |
| **Automation (P5)** | Trusts the carry because they set up the standing content. Wants it fast and quiet. | Didn't set up anything. Can't trust what he can't see. Quiet automation is invisible, and invisible automation is the risk. |
| **Forgiveness (P6)** | Mistakes are small and noticed quickly. Undo and autosave cover them. Two-click arms are fine because they're rare. | Mistakes are discovered late, often on Sunday. Undo doesn't help after printing. Forgiveness for him means catching it before print, not reversing it after. |
| **Recall vs. recognition (P3)** | Will use keybindings. Fine, since presets only set defaults. | Will remember nothing. Must never need a shortcut or a remembered setting. |
| **Check** | Wants a short list and done. | Wants confidence, because his name is on it. Wants to know what was checked, not only what failed. |
| **Printer (P6, P9)** | One printer, answers remembered. | Unknown printer, no answers, doesn't know the vocabulary of duplexing. |

**How to serve both without two apps.** Scale the explanation by *staleness*, not by *role*. No modes, no "expert" switch. The same screens, carrying more or less explanation depending on two facts the app already has: how long since this machine opened a bulletin, and how old the source file is.

- **Same five steps for both.** Each step shows its state ("done by Next Sunday," "needs you," "checked"). The admin reads that as a status bar and skips. The pastor reads it as instructions.
- **Same change summary after Next Sunday.** The admin skims three lines. The pastor reads all of them. Nothing hides it from either.
- **Same Check list.** Add a "what was checked" line under the findings. One line costs the admin nothing and gives the pastor the confidence he needs.
- **Same staleness warning.** Warn about the source file's date when it's older than a week. It only fires for the person who needs it. The admin opening last Sunday's file never sees it.

---

## 4. The five changes I would make first, in order

**1. Start names the Sunday being made, and proposes Next Sunday when the last bulletin is done.**
- **The change:**
  - Start says in plain words: "Last bulletin: Sunday Oct 11 (print copy made Oct 9)."
  - The primary action becomes "Next Sunday: make Oct 18 from Oct 11" whenever the last bulletin has a print copy.
  - Before anything is created, it states the target date computed from today's date.
  - If the source is more than a week old, or isn't the newest bulletin it knows of, it says so before acting.
- **Restores:** P4 (Feedback, next step visible) and P5 (automation proposes, human fires).
- **Fixes:** F1 and F5. It also turns the admin's habit into the correct path instead of fighting it.

**2. Check gains weekly-content findings.**
- **The change:** New plain-word findings, each with the same jump button:
  - "Hymn number is empty."
  - "Readings are the same as last week."
  - "Sunday name didn't change but the date did."
  - "Sermon title is empty."
- Below the list, one line names what was checked, so that "no findings" means something.
- **Restores:** P6 (never surprise) and P5 (the human fires the consequential act, informed).
- **Fixes:** F2.

**3. The print handoff tells the person what to pick in the dialog they're about to see.**
- **The change:** After "Open the print copy," show a short note built from the remembered two-sided answers: "In the print window choose: Actual size (not Fit). Two-sided: flip on short edge. Print page 1 only first, check it, then print the rest."
- On an unremembered printer, ask the two-sided questions *before* making the print copy, with a one-sheet test as the recommended first step.
- **Restores:** P6 (never surprise at the printer) and P9 (conform to the worker, meeting them where the printing actually happens).
- **Fixes:** F3 and part of F12.

**4. Next Sunday ends with a change summary that stays reachable.**
- **The change:** Immediately after Next Sunday, show:
  - date advanced: Oct 11 → Oct 18
  - cleared: sermon title, readings, hymns, 2 announcements
  - kept: 3 announcements, each "through Nov 1"
  - expiring after this Sunday: 1
- Each line jumps to its box. The summary stays visible in the This Sunday step until the print copy is made.
- **Restores:** P6 (Perceived Stability: nothing vanishes unreported) and P4 (Feedback).
- **Fixes:** F4. It also gives the pastor the "what did the machine do" answer he needs.

**5. Opening an old bulletin ends by asking which text changes weekly, answered by pointing.**
- **The change:** After an import is opened, one guided pass asks: "Point at the date. Point at the readings. Point at the hymns. Point at each announcement." Each click binds that box to a field, or marks it standing or weekly. It is one question per thing, with the page itself as the answer surface. This is the hand-driven version of what the Template Analyzer promises, and it doesn't wait for it.
- **Restores:** P5 (automate the recurring, only possible once text is a field), P7 (imports are rescues a human accepts), and P9 (grab the thing itself).
- **Fixes:** F6, the week-one abandonment risk.

**Cheap and alongside:** a vocabulary sweep for F9 ("story" → "announcement"; drop "encoding" from findings; review "print copy" and "panel" with real admins), and a written decision on F10.

**Next in line:** in-place caret editing (F7), already recorded as debt.

---

## 5. Questions for the pastor interview

These aim at what documents can't tell us: how mistakes actually happen, who does what, and what words he uses.

**Mistakes and accountability**
1. What is the worst bulletin mistake you remember in 30 years? Who caught it, and when: before printing, during the service, after?
2. Which errors do people actually mention to you afterward? Dates, hymn numbers, names, readings, typos?
3. What do you check yourself before it prints, and what do you just trust?

**What changes, and when**
4. Walk me through the bulletin, part by part. Which parts change every week, which change by season, and which never change?
5. Who decides an announcement comes off? How long do they usually run? Has one ever disappeared too early or stayed too long?
6. Do you plan sermon titles and readings weeks ahead? Where do you keep them? When does the office get them?
7. Is there a prayer list or names of the sick? Who may be listed, and how do names come off?
8. How many bulletins a year are not a regular Sunday: Christmas Eve, Ash Wednesday, Good Friday, funerals, weddings? Does the layout change for Advent and Lent?

**The handoff**
9. When the administrator is away, what actually happens today, step by step? Whose computer, where are the files, who prints?
10. When admins have changed over the years, how was the job passed on? What got lost?
11. What computer do you use, and have you ever opened Publisher yourself?

**Proofing**
12. How do you proof now: paper, a PDF in email, over the admin's shoulder? How many rounds? What's the latest a change is allowed?
13. When you mark a correction, how do you point at it? Line number, circled words, a phone call?
14. Has a corrected draft ever been printed from the old version?

**Printing**
15. Who prints the 16-page booklet, on what machine, how many copies, and when? Copier from a master, or straight from the computer?
16. What goes wrong at the printer: upside-down backs, shrunk pages, wrong order, jams, folding?
17. When you print a proof at home or another office, do you know how to make it two-sided? What do you do if the printer can't?
18. Has anything ever had to be changed Sunday morning, such as a death or weather? What happened to the printed bulletins?

**Words and content**
19. What do you call the parts? Cover, order of worship, insert, back page, announcements? What does the admin call them?
20. Which hymnal or hymnals do you cite? Is it always number only, or are words ever printed under a license?
21. Do you or the congregation keep old bulletins? Have you ever needed to find last year's Easter bulletin?
22. What would let you trust the printed bulletin without reading every line, if anything?