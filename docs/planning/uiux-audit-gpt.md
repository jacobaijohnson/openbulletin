### 1. User journey, step by step – what the admin actually meets

I’ll walk the flow as the code defines it, then call out issues with severity and evidence.

---

## First launch + Start step (“Bulletin shape”)

**What the user sees**

On first launch `App::default()` builds a template for “Grace Lutheran Church” with today’s date:

```rust
let church_name = "Grace Lutheran Church".to_string();
let preset = Preset::LetterBifold;
let doc = build_template(
    preset,
    ChurchProfile {
        name: church_name.clone(),
        service_times: "Sundays 9:30 AM".into(),
        ..Default::default()
    },
    chrono::Local::now().date_naive(),
    "Sunday",
);
self.status = "Welcome to OpenBulletin.".into();
self.step = 0;
```

Top bar:

```rust
ui.heading("OpenBulletin");
if ui.button("Pages").clicked() { ... }
if ui.button("Fonts").clicked() { ... }
...
ui.label(format!("Step {} of 5: {}", self.step + 1, STEPS[self.step]));
...
if self.step < 4 && ui.button(
    egui::RichText::new(format!("Next: {}", STEPS[self.step + 1])).strong()
).clicked() { ... }
...
ui.label(GUIDANCE[self.step]);
```

With `STEPS[0] = "Start"` and:

```rust
GUIDANCE[0] =
"Pick the bulletin shape and confirm your church name. Everything else starts from last week's file."
```

Left side panel for step 0:

```rust
ui.heading("Bulletin shape");
for p in Preset::all() {
    if ui.selectable_label(self.preset == p, p.label()).clicked() {
        self.preset = p;
        self.rebuild();
    }
}
ui.separator();
ui.label("Church name");
if ui.text_edit_singleline(&mut self.church_name).lost_focus() {
    let name = self.church_name.clone();
    self.doc.profile.name = name;
}
```

Central panel is a preview:

```rust
ui.label(egui::RichText::new("Preview: what the congregation will hold").strong());
```

### Flow & language

- The next step is clearly visible: top-right “Next: This Sunday”.
- Language on this screen is mostly ELI5: “Bulletin shape”, “Church name”.
- The central preview reinforces “this is what people will hold” in plain words.

### Findings

1. **No way to set basic church profile beyond name.**

   - Only `profile.name` is editable:

     ```rust
     self.doc = build_template(... ChurchProfile { name: church_name.clone(), service_times: "Sundays 9:30 AM".into(), ..Default::default() }, ...);
     ...
     ui.label("Church name");
     if ui.text_edit_singleline(&mut self.church_name).lost_focus() {
         let name = self.church_name.clone();
         self.doc.profile.name = name;
     }
     ```

   - There is **no UI** for service times, address, pastor, website, etc., despite the engine using `ChurchProfile` for cover and order-of-service fields.

   **Severity: BLOCKER**

   For a church office admin, printing a bulletin with the wrong service time or missing address is not acceptable, and there’s no in-app way to correct it without editing JSON or using the CLI. That fails “conform to the office worker” and “simple on the surface”.

2. **Guidance line implies “last week’s file” exists, but the first-run path does not.**

   ```rust
   GUIDANCE[0] =
   "Pick the bulletin shape and confirm your church name. Everything else starts from last week's file.";
   ```

   On first launch there *is* a new template (from `App::default()`), not last week’s file. The admin hasn’t opened anything yet.

   **Severity: FRICTION**

   This can confuse first-time users: they may think they must somehow locate “last week’s file” before continuing, even though a working template is already present.

3. **No “New bulletin” concept separate from “Open bulletin”.**

   - Top bar only has:

     ```rust
     if ui.button("Open bulletin...").clicked() { ... }
     if ui.button("Save bulletin").clicked() { ... }
     ```

   There is no button labelled “New bulletin” or “Start a new bulletin”. First launch silently *is* a new bulletin, but that’s implicit.

   **Severity: FRICTION**

   Office users expect a visible “New…” choice. It’s not a hard blocker (they can just start editing), but the mental model (“am I editing something already?”) is fuzzy.

---

## Step 1 – “This Sunday”

Side panel:

```rust
ui.heading("This Sunday");
ui.label(format!(
    "Date: {}",
    self.doc.fields.get("sunday_date_long").cloned().unwrap_or_default()
));
ui.label("Name of the Sunday");
let mut sn = self.doc.fields.get("sunday_name").cloned().unwrap_or_default();
if ui.text_edit_singleline(&mut sn).lost_focus() {
    self.doc.fields.insert("sunday_name".to_string(), sn);
}
ui.label("Scripture reading");
let mut rd = self.doc.fields.get("reading").cloned().unwrap_or_default();
if ui.text_edit_singleline(&mut rd).lost_focus() {
    self.doc.fields.insert("reading".to_string(), rd);
}
ui.small("The cover and the order of service fill themselves from these.");
```

Guidance:

```rust
GUIDANCE[1] =
"Confirm the date and the name of the Sunday. These fill the cover automatically.";
```

### Flow & language

- “Next: Announcements” is visible in the top bar.
- “Name of the Sunday” and “Scripture reading” are plain.
- The guidance explicitly mentions “Confirm the date…”, but:

### Findings

4. **The date is shown but not editable.**

   - Date is just a label:

     ```rust
     ui.label(format!(
         "Date: {}",
         self.doc.fields.get("sunday_date_long").cloned().unwrap_or_default()
     ));
     ```

   - There is no text field or date picker here.
   - `build_template` set the date once at app start:

     ```rust
     chrono::Local::now().date_naive()
     ```

   - `run_checks` looks for a missing date:

     ```rust
     match self.doc.fields.get("sunday_date_long") {
         Some(d) if !d.is_empty() => notes.push(format!("OK: bulletin date is {d}.")),
         _ => notes.push("Check: no Sunday date is set (Step 2).".to_string()),
     }
     ```

   But the date is never empty, and can’t be corrected if wrong.

   **Severity: BLOCKER**

   A weekly bulletin must have the correct Sunday. There is no in-app way to change it. The admin would have to use the CLI or edit the `.obd` JSON, which violates the “administrator, not designer/technician” principle.

5. **The “Confirm the date” guidance is misleading given read-only UI.**

   ```rust
   GUIDANCE[1] =
   "Confirm the date and the name of the Sunday. These fill the cover automatically.";
   ```

   But there is literally nothing they can do if the date is *not* the upcoming Sunday.

   **Severity: FRICTION**

   This will cause frustration and a sense of powerlessness: the UI asks them to confirm something they cannot change.

6. **No explanation of how “Next Sunday” relates to the date.**

   - The only visible place “Next Sunday” appears is in the Print step:

     ```rust
     if ui.button(egui::RichText::new("Start next Sunday").strong()).clicked() {
         self.next_sunday();
     }
     ```

   - After that, status:

     ```rust
     self.status = format!(
         "Started {} bulletin. Weekly announcements were cleared; standing items carried over.",
         self.doc.fields.get("sunday_date_long").cloned().unwrap_or_default()
     );
     ```

   There’s no guidance in Step 1 like “If this date is wrong, click ‘Start next Sunday’ in Print” or a direct “Use next Sunday” button here.

   **Severity: FRICTION**

   A first-time admin prepping mid-week may never discover that the way to fix the date is by going all the way to Print and pressing “Start next Sunday”.

---

## Step 2 – “Announcements”

Side panel:

```rust
ui.heading("Announcements");
ui.label("One announcement per line. Paste from email works; formatting is cleaned up for you.");
ui.add(
    egui::TextEdit::multiline(&mut self.announcements)
        .desired_rows(12)
        .desired_width(f32::INFINITY),
);
if ui.button("Save announcements").clicked() {
    self.save_announcements();
    self.status = "Announcements saved.".into();
}
```

Guidance:

```rust
GUIDANCE[2] =
"Type or paste this week's announcements. Standing items (service times, the order of service) carry over on their own.";
```

Top bar automatically saves when they click “Next: Check”:

```rust
if self.step == 2 {
    self.save_announcements();
}
self.step += 1;
if self.step == 3 {
    self.run_checks();
}
```

### Flow & language

- The next step (“Next: Check”) is clearly presented.
- Instructions are ELI5: “One announcement per line”, “Paste from email works”.
- They don’t have to remember to hit “Save announcements”; the step transition does it.

### Findings

7. **Announcements experience is solid but hidden coupling may surprise:**

   `save_announcements()` silently rewrites the story structure:

   ```rust
   if !inserted && p.style == "subheading" && p.text == "Announcements" {
       for l in &lines {
           new_paras.push(ob_model::Paragraph::new("body", l));
       }
       inserted = true;
   }
   ```

   The user never sees that the underlying “Announcements” subheading is the anchor. If the template is edited later and that heading changes text/style, this logic breaks, with no explanation.

   **Severity: POLISH**

   Today the shipped template will likely match, so it works; but explaining “these lines go under the ‘Announcements’ heading” in UI would make the behavior less magical.

---

## Step 3 – “Before you print” (Check step)

Side panel:

```rust
ui.heading("Before you print");
for n in &self.notes {
    ui.label(n);
}
if ui.button("Check again").clicked() {
    self.save_announcements();
    self.run_checks();
}
```

`run_checks()` produces plain-language notes:

- Church name:

  ```rust
  if self.doc.profile.name.trim().is_empty() {
      notes.push("Check: the church name is blank (Step 1).".to_string());
  } else {
      notes.push("OK: church name is set.".to_string());
  }
  ```

- Date:

  ```rust
  match self.doc.fields.get("sunday_date_long") {
      Some(d) if !d.is_empty() => notes.push(format!("OK: bulletin date is {d}.")),
      _ => notes.push("Check: no Sunday date is set (Step 2).".to_string()),
  }
  ```

- Announcements:

  ```rust
  if self.announcements.trim().is_empty() {
      notes.push("Check: announcements are empty. If that is correct for this week, you can ignore this.".to_string());
  } else {
      notes.push("OK: announcements are filled in.".to_string());
  }
  ```

- Text overflow:

  ```rust
  fit_notes.push(format!(
      "Check: the words in \"{}\" ({}) do not all fit. About {} line{} too many. Shorten the words, or we can shrink them in a later version.",
      area.chars().take(38).collect::<String>(),
      panel.name,
      over_lines,
      if over_lines == 1 { "" } else { "s" }
  ));
  ```

- Order of service:

  ```rust
  if has_order {
      notes.push(
          "OK: order of service is present and carries over from last week.".to_string(),
      );
  } else {
      notes.push("Check: no order of service found.".to_string());
  }
  ```

Guidance:

```rust
GUIDANCE[3] =
"We check the things that usually go wrong: missing date, empty announcements, missing church name.";
```

### Flow & language

- Next step (“Next: Print”) is visible.
- Messages are plain, specific, and reference the step where to fix (“Step 1”, “Step 2”).
- The overflow message (“About X lines too many. Shorten the words…”) is strong ELI5.

### Findings

8. **No direct “jump to fix” from a check item.**

   Checks reference steps (“Step 1”, “Step 2”), but fixing involves manually clicking “Back” and then finding the right field.

   **Severity: POLISH**

   Usable as-is, but adding small “Go to Start” / “Go to Announcements” buttons next to problem lines would reduce hunting and better align with “guided flow” / “one visible next step”.

9. **Overflow guidance mentions future capability (“we can shrink them in a later version”).**

   ```rust
   "Shorten the words, or we can shrink them in a later version."
   ```

   At v0.1 that shrinking does not exist.

   **Severity: POLISH**

   The sentence is honest (“later version”), but could raise expectations or cause confusion.

---

## Step 4 – “Print”

Side panel (simplified):

```rust
ui.heading("Print");
self.save_announcements();
self.sheet_preview_ui(ui);
ui.separator();
ui.label(egui::RichText::new("In the print window, use these settings").strong());
for line in self.print_checklist() {
    ui.label(format!("- {line}"));
}
...
if ui.button("Open the print copy").clicked() { ... }
...
if ui.button(egui::RichText::new("Save print copy (PDF)").strong()).clicked() { ... }
...
if ui.button("Save reading copy (PDF)").clicked() { ... }
...
ui.label(egui::RichText::new("Two-sided printing, first time on a printer?").strong());
ui.label("Every printer flips the paper its own way. Print one test sheet and tell us how it came out. We remember the answer for that printer.");
...
if ui.button("Save a test sheet (PDF)").clicked() { ... }
...
ui.label("Hold the FRONT so you can read it. Turn the sheet like a book page. The BACK was:");
ui.horizontal(|ui| {
    for (answer, label) in [
        ("up", "Right side up"),
        ("down", "Upside down"),
        ("manual", "My printer does one side at a time"),
    ] {
        if ui.button(label).clicked() {
            let setup = printer_setup_from_answer(answer, &self.printer_name);
            self.settings.printers.insert(self.printer_name.clone(), setup);
            match save_settings(&self.settings) {
                Ok(()) => self.status = format!("Remembered for the {}. You will see the setting to use right here.", self.printer_name),
                Err(e) => self.status = format!("Learned it, but could not save the note: {e}"),
            }
        }
    }
});
...
if ui.button(egui::RichText::new("Start next Sunday").strong()).clicked() {
    self.next_sunday();
}
```

`print_checklist_for()` creates plain instructions:

```rust
lines.push(format!("Paper: {}", paper_name(w, h)));
lines.push("Size: pick Actual size, or 100%. Never \"Fit to page\": it shrinks \
         the pages and the fold lands in the wrong place."
            .to_string());
...
```

Guidance:

```rust
GUIDANCE[4] =
"Print the folded bulletin, or save a reading copy as PDF. Then start next Sunday in one click.";
```

### Flow & language

- The Print step’s purpose is crystal clear.
- The “Start next Sunday” button is exactly the “one visible next step after print” principle.
- Duplex learning (“Hold the FRONT… The BACK was: Right side up / Upside down / My printer does one side at a time”) is ELI5 and practical.

### Findings

10. **Print and PDF error messages expose raw OS errors.**

   Examples:

   ```rust
   Err(e) => self.status = format!("Could not save the print copy: {e}");
   ...
   .map_err(|e| format!("could not open the print copy: {e}"))
   ...
   Err(e) => self.status = format!("Could not save the test sheet: {e}");
   ...
   Err(e) => self.status = format!("Learned it, but could not save the note: {e}");
   ```

   `e` may be “Permission denied”, paths, etc.

   **Severity: FRICTION**

   The admin *can* still work around (e.g., choose another folder), but the messages are halfway between plain language and system jargon. They don’t suggest next steps (“Try saving in Documents”).

11. **“Open the print copy” vs “Save print copy” could be confusing.**

   - “Open the print copy” writes a temp file and opens it:

     ```rust
     let path = std::env::temp_dir().join("openbulletin-print-copy.pdf");
     ...
     self.status = "Print copy opened in your PDF viewer. Use the settings listed above.".into();
     ```

   - “Save print copy (PDF)” asks for a path.

   **Severity: POLISH**

   Functionally fine, but the distinction may not be obvious. A label like “Open a one-time print copy” or “Save a PDF you can reuse later” would help.

---

## Pages canvas window (“Pages”)

Accessed from top bar:

```rust
if ui.button("Pages").clicked() {
    self.show_pages = true;
}
...
self.pages_window(ctx);
```

Window:

```rust
egui::Window::new("Pages")
    .open(&mut open)
    .default_width(720.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("<").clicked() && self.canvas_panel > 0 {
                self.canvas_panel -= 1;
            }
            let panel = &self.doc.panels[self.canvas_panel];
            ui.label(egui::RichText::new(format!(
                "{}  ({} of {})",
                panel.name,
                self.canvas_panel + 1,
                self.doc.panels.len()
            )).strong());
            if ui.button(">").clicked()
                && self.canvas_panel + 1 < self.doc.panels.len()
            {
                self.canvas_panel += 1;
            }
        });
        ui.label("Click a box to select it. Drag to move it, drag its corner square to size it, arrow keys nudge it. Double-click a box to type in it. Boxes and guides both snap to the blue lines. Right-click anything for its exact place in inches. Guides never print.");
        ...
        ui.separator();
        self.panel_canvas(ui, idx, ctx);
        ui.separator();
        let n = self.doc.panels[idx].guides.len();
        ui.small(format!(
            "{n} guide{} on this page. They save with the bulletin.",
            if n == 1 { "" } else { "s" }
        ));
    });
```

Inside `panel_canvas`, the user gets:

- Inch rulers:

  ```rust
  // Rulers: quarter-inch ticks, a number each inch.
  ```

- Boxes (frames) with a blue selection and corner handle:

  ```rust
  ui.label("Click a box to select it. Drag to move it...");
  ...
  painter.rect_stroke(...);
  ```

- Snap behavior to guides and edges, arrow key nudges.
- Double-click to type:

  ```rust
  if response.double_clicked() {
      if let Some(pos) = pointer {
          ...
          if self.doc.panels[idx].frames[fi].story_id.is_some() {
              self.enter_edit(idx, fi);
          }
      }
  }
  ```

- Right-click context menu for precise positioning:

  ```rust
  ui.label("Exact place and size, in inches:");
  ...
  if ui.button("Set place and size").clicked() { ... }
  ...
  ui.label("Exact place, in inches (1.25 or 1 1/4 works):");
  ...
  ui.button("Delete this guide")
  ...
  ui.button(format!("Add vertical guide here ({:.2} in)", ...))
  ui.button(format!("Add horizontal guide here ({:.2} in)", ...))
  ```

- Inline typing bar when editing:

  ```rust
  ui.horizontal(|ui| {
      ui.label(egui::RichText::new(format!("Typing in: {frame_name}")).strong());
      ui.label("This paragraph:");
      egui::ComboBox::from_id_salt("para-style")
          .selected_text(cur_name)
          .show_ui(ui, |ui| { ... });
      if ui.button("Done typing").clicked() {
          self.edit_done = true;
      }
  });
  ```

### Flow & language

- Instructions at the top are excellent ELI5 and match Publisher habits: clicking boxes, dragging, right-clicking for precise inches.
- Guides are named “guides” but explained sufficiently (“Guides never print.”).
- The typing experience is inline, with a visible “Done typing” button.

### Findings

12. **Concept of “guides” and “box” is clear; “panel” and “frame” internal terms are kept out of UI – good.**

   No issue: you’ve conformed well to office-worker vocabulary here.

13. **Inline editor’s focus behavior is a bit magical but mostly invisible.**

   - The code manages `self.focus_edit` and text cursor paragraph tracking via `cursor_paragraph()`. None of that leaks into UI, so the admin just sees “Done typing” working.

   **Severity: POLISH**

   Usable as-is. No direct user-facing issue.

14. **No obvious link from Check step to Pages window when text doesn’t fit.**

   - Overflow message:

     ```rust
     "Check: the words in \"...\" (...) do not all fit. About {} line{} too many. Shorten the words, or we can shrink them in a later version."
     ```

   - But nowhere does it say “Open the Pages window and resize that box”.

   **Severity: FRICTION**

   Users who think in “make the box bigger” rather than “shorten words” may not discover the Pages window as the tool to do that.

---

## Fonts window (“Fonts”)

Access from top bar:

```rust
if ui.button("Fonts").clicked() {
    self.show_fonts = true;
}
...
self.fonts_window(ctx);
```

Window content (abridged):

```rust
egui::Window::new("Fonts")
    ...
    .show(ctx, |ui| {
        ui.label(
            "These are the fonts this app may use. Bulletins are laid out                      only with fonts from this library, so they look the same on                      every computer.",
        );
        ...
        for row in &self.font_rows {
            ui.label(
                egui::RichText::new(
                    "Welcome! The Lord be with you. 1234567890",
                )
                .font(egui::FontId::new(
                    17.0,
                    egui::FontFamily::Name(row.key.clone().into()),
                )),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} ({})",
                    row.full_name, row.family
                ))
                .strong(),
            );
            ui.label(egui::RichText::new(&row.rights_words).small());
            ui.separator();
        }
        ...
        ui.label(egui::RichText::new("Get a free font").strong());
        ui.label(
            "Search the free font catalog (nearly all of it is free to \
             use and share). Getting a font adds its regular, bold, \
             and italic faces to this library.",
        );
        ...
        ui.label(egui::RichText::new("The Church Starter Pack").strong());
        ui.label(
            "One tap adds the free stand-ins for the fonts church \
             computers already have: Carlito (for Calibri), Caladea \
             (Cambria), Gelasio (Georgia), plus Lora for readings \
             and Playfair Display for titles and holidays.",
        );
        ...
        ui.label(egui::RichText::new("Fonts on this computer").strong());
        ui.label(
            "OpenBulletin keeps its fonts in its own library, so a \
             bulletin looks the same on every computer. To use a \
             font your computer already has, add a copy of it here.",
        );
        ...
    });
```

Status messages on adding fonts:

```rust
self.status = if added.is_empty() {
    format!("No fonts added. {}", problems.into_iter().next().unwrap_or_default())
} else {
    format!("Fonts added: {}{}", added.join(", "), caution)
};
```

### Flow & language

- First paragraph explains “fonts this app may use… looks the same on every computer” – good.
- Actions are straightforward: “Add fonts…”, “Search”, “Install the Starter Pack”, “Show fonts on this computer”, “Add a copy”.

### Findings

15. **First label contains visual noise (extra spaces) and is very long.**

   ```rust
   "These are the fonts this app may use. Bulletins are laid out                      only with fonts from this library, so they look the same on                      every computer."
   ```

   The embedded spaces and length may reduce readability.

   **Severity: POLISH**

   Splitting this into two short sentences on separate lines would better match KISS.

16. **Error messages in the font catalog are technical.**

   Examples:

   ```rust
   self.status = format!("Could not open the font library: {e}");
   ...
   self.status = format!("Font catalog: {e}");
   ...
   self.status = format!("Could not add that font: {e}")
   ```

   **Severity: FRICTION**

   Office admins don’t know what to do with “Font catalog: network error” or similar. A follow-up hint (“Check your internet connection and try again”) would help.

---

## Recovery dialog and autosave

Autosave journal:

```rust
fn journal_tick(&mut self) {
    ...
    self.save_announcements();
    let snap = snapshot(&self.doc);
    if snap == self.saved_snapshot {
        ...
        clear_journal_from(&obd, &meta);
        ...
    }
    ...
    if write_journal_to(&obd, &meta, &self.doc, &self.path).is_ok() {
        self.journal_snapshot = snap;
    }
}
```

Recovery window:

```rust
egui::Window::new("Your work from last time")
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let clock = ...;
        ui.label(format!(
            "OpenBulletin kept a copy of the bulletin you were editing, \
             saved on {clock}. It was never saved to a file."
        ));
        ui.horizontal(|ui| {
            if ui.button("Bring it back").clicked() {
                bring_back = true;
            }
            if ui.button("Throw it away").clicked() {
                throw_away = true;
            }
        });
    });
...
self.status = "Your unsaved work is back. Save when ready.".into();
...
self.status = "Threw away the unsaved copy.".into();
```

### Flow & language

- The autosave strategy is robust: every ~2 seconds, changes are journaled.
- Recovery text is exemplary ELI5: explains what happened, when, and offers two clear choices.

### Findings

17. **Recovery window is good; no UX blocker.**

   No changes needed here; it closely meets “Never lose work”.

---

## Status messages (global)

Status is shown in the side panel:

```rust
ui.separator();
ui.small(&self.status);
```

Examples of plain messages:

- `"Welcome to OpenBulletin."`
- `"Announcements saved."`
- `"Bulletin opened."`
- `"Saved to {path.display()}"`
- `"Text updated."`
- `"Box moved and sized."`
- `"Guide moved."`
- `"Guide deleted."`
- `"Your unsaved work is back. Save when ready."`

Examples mixing plain text and raw errors:

- `"Could not save: {e}"`
- `"Could not open that file: {e}"`
- `"Next Sunday failed: {e}"`
- `"Could not save the print copy: {e}"`
- `"could not open the print copy: {e}"`
- `"Could not save the test sheet: {e}"`
- `"Could not open the font library: {e}"`
- `"Font catalog: {e}"`

### Findings

18. **Error messages consistently expose raw technical error strings.**

   **Severity: FRICTION**

   In everyday failure (network issues, permission errors), users get no guidance beyond “Could not …”. It’s usable, but not comforting or instructive for non-technical staff.

---

## 3. Overall completeness for the target user

Assuming the engine behaves as written, **is this end-to-end usable by a church office administrator?**

Strengths:

- Guided five-step flow with a visible “Next” on each step and “Start next Sunday” at the end.
- Clear Check step that catches very real failure modes (missing name, missing date, empty announcements, text overflow).
- Robust autosave and friendly recovery dialog.
- Pages window closely matches Publisher habits (inch rulers, right-click for exact inches, snap-to-guides, inline typing).
- Print step duplex guidance is unusually good for this audience.

Critical gaps:

1. **No UI to set the bulletin date.**  
   The date is fixed at app launch (`chrono::Local::now().date_naive()`), displayed but uneditable. This violates the core weekly workflow: preparing a bulletin for a *future* Sunday.

2. **No UI to set the full church profile (service times, address, pastor, etc.).**  
   These are central to the bulletin and referenced in docs, but only `profile.name` surfaces in the Start step.

3. **The “last week’s file” concept is not actually wired into the flow.**  
   The guidance says “Everything else starts from last week's file”, but the app starts on a fresh template, and there is no explicit “Use last week’s bulletin as a starting point” button. The “Open bulletin…” and “Start next Sunday” commands can do this, but the path is implicit.

4. **Corrections for layout/overflow are text-centric (“shorten words”), not layout-centric (“resize the box”), and the UI doesn’t connect the Check step to the Pages window.**

5. **Error messages don’t provide plain guidance when something goes wrong.**

Given these, I’d answer:

- **No, this is not yet fully usable by its target user without assistance.**  
  A technically comfortable user could work around by using the CLI or editing JSON, but a typical church office admin will be blocked by the inability to set the date and core profile data.

---

## 4. Ordered fix list (most valuable first)

Each item is 1–2 lines, focused on UI/UX.

1. **Add editable date in “This Sunday” step.**  
   Replace the static “Date: …” label with a text field (and optional date-picker) bound to `sunday_date_long` / the underlying date field, with guidance like “Type the Sunday’s date (October 18, 2026).”

2. **Expose main ChurchProfile fields in the Start step.**  
   Under “Church name”, add text fields for “Service times”, “Address”, “Pastor”, “Website”, wiring them to `doc.profile` so the cover and order-of-service show correct information without CLI use.

3. **Make the “last week’s file” path explicit and simple.**  
   Add a visible “Use last week’s bulletin…” button on Start that opens a `.obd` and immediately runs `next_sunday`, and revise `GUIDANCE[0]` to match that reality.

4. **Connect Check step findings to actions.**  
   Next to each “Check:” note, add small buttons like “Go to Start”, “Go to Announcements”, or “Open Pages for this panel” so the fix is one click away.

5. **Point overflow problems toward the Pages window.**  
   Extend overflow messages to say “Open the Pages window and make that box bigger, or shorten the words.”

6. **Improve guidance around the date and “Start next Sunday”.**  
   In Step 1, explain how “Start next Sunday” changes the date and add an inline “Use next Sunday” button that calls `next_sunday()` without making users travel to Print.

7. **Wrap technical error messages in plain-language hints.**  
   Replace raw `{e}` exposure with friendly sentences like “Could not save the PDF. Try saving in your Documents folder. (Details: {e})”.

8. **Refine Fonts window copy for readability.**  
   Split the long first label into two lines, remove extra spaces, and keep explanations short and direct for non‑designers.

9. **Clarify “Open print copy” vs “Save print copy”.**  
   Slightly adjust labels and guidance: e.g., “Open a one-time print copy (for this run)” vs “Save a print-ready PDF (for reuse or archiving).”

10. **Add a visible “New bulletin” command.**  
    Provide an explicit “New bulletin” button that builds a fresh template (with date set to upcoming Sunday once date editing is supported), reassuring users they are not editing an old file.