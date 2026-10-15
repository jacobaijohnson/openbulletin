# Closing audit, Tier-2 gpt (2026-10-15)

Milestone judgment on the closing synthesis, the security report, and the effectiveness report. Usage: {'prompt_tokens': 11135, 'completion_tokens': 2604, 'total_tokens': 13739}.

---

1. **READINESS**

No: I would not hand this to even one real church office administrator for a supervised first Sunday yet.

The single strongest reason is **E‑B1 from the effectiveness lane**: the Step 1 “Use last week’s bulletin…” button:

- Bypasses the unsaved‑work guard that the top‑bar “Open bulletin…” uses.
- Immediately adopts the picked file and calls `mark_clean()`, which clears the autosave journal (`crates/ob-app/src/main.rs:2683–2696`).
- There is no dirty check and no warning.

This means a very plausible workflow (typing into a fresh bulletin, then clicking “Use last week’s bulletin…” as the first proper step) silently destroys the work and its recovery copy, in the *first button of the weekly journey*, directly contradicting Principle 6 (“Never lose work…”). Even with someone nearby who knows it’s early software, that one‑click destructive path on the main happy path is too risky for a real admin.

E‑B2 (stale edit index on Delete, leading to panic or wrong‑story overwrite) and E‑M1 (Save/Print ignoring open typing sessions) reinforce the same “lose or corrupt work” concern, but E‑B1 alone is enough to keep the answer at “no” today.


2. **BLIND SPOTS / MOST VALUABLE REMAINING CHECKS**

Even with six blind lanes and hostile‑input testing, there are structural things this process can’t see without a different kind of check or a different model family:

1. **Actual interactive use of the GUI by a human admin.**  
   The synthesis explicitly notes “The GUI has still never been run by a human… The remaining honest gate is a person opening the app and making a bulletin.” All lane judgments (including the effectiveness lane’s journey walk) are based on code reading and tests, not lived interaction. A different model family (or just a different methodology) should:

   - Run the app on a real Windows church‑style machine.
   - Observe a real office administrator doing the weekly flow (Start → This Sunday → Announcements → Check → Print).
   - Watch for commit‑on‑blur issues, focus traps, confusing wording, and emergent behaviors that aren’t obvious from code (e.g., E‑m1’s date‑field behavior, E‑m4’s Pages window navigation).

   This is the single most valuable remaining check.

2. **Live print/preview and performance on actual office hardware.**  
   Efficiency lane originally found serious issues with Booklet and Check on an old office PC; the fixes refactored font shaping and caching. But there has still been no in‑situ verification that:

   - “Print output matches the preview” under real printer drivers, duplex settings, and paper sizes (Principle 6).
   - The updated shaper and Check step feel responsive on a low‑spec Windows box.

   A different model family could drive scenario‑based tests: create and print a booklet on a typical 4–8 GB church PC, confirm timing and preview/print fidelity across several printers.

3. **Independent review of the fix diff around autosave/undo/state.**  
   The coordinator who wrote the original bugs also wrote the fixes, under time pressure. The effectiveness lane has already found that one of the most critical fixes—the autosave/recovery behavior—is still flawed (E‑M4), despite the synthesis claiming the bug was closed. That suggests cognitive blind spots around state invariants. A fresh pair of eyes (or a different model family) should:

   - Re‑audit the code changes in the autosave, recovery, undo/redo, and edit‑session areas.
   - Check explicitly for invariants: “no journal deletion while recovery is pending,” “no stale indices,” “all user‑visible outputs commit edits first.”

   This would likely surface subtle regressions that structural lane separation doesn’t catch when the same person wrote both bug and fix.


3. **FIX‑ROUND RISK: MOST DESERVING OF A SECOND LOOK**

The fix that most needs a second look is the **autosave journal / recovery decision logic**.

The synthesis states that the catastrophic behavior where autosave deleted the only recovery copy “is fixed (the journal decision is now a pure, tested function that holds while a recovery is undecided…)”. However, the effectiveness lane’s **E‑M4** shows the current implementation still deletes the recovery copy while the recovery decision is pending:

- `startup()` detects a leftover journal and sets `self.recovery` for the dialog (`crates/ob-app/src/main.rs:860–875`).
- `journal_tick()` (`877–898`) checks snapshots and, when the frame equals the saved snapshot, clears the journal files.
- It does this without consulting `self.recovery`; so within a couple of seconds of launch—while the recovery dialog is still up—the autosave tick can remove the on‑disk recovery journal.

If the app or machine dies while the user is still deciding, the copy the dialog is offering no longer exists on disk. That directly violates Principle 6 and contradicts the synthesis’ claim that this path is fixed.

Because this logic is central to “never lose work,” crosses multiple states (startup, recovery dialog, periodic autosave), and already shows a discrepancy between the synthesis and observed behavior, it is the fix that most urgently needs a focused re‑audit and possibly a redesign around clear invariants (e.g., “journal_tick must be a no‑op while `self.recovery.is_some()`”).