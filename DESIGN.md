# Design

The standing record of UI decisions. Claude Design reads the repo's tokens and this
file before drawing; it never writes a record of its own, so every design session
ends by adding a row to the Decisions table below. The brief for the current design
effort is `docs/design/v1-brief.md`; the turn-two brief is `docs/design/v1-turn-2.md`
and the turn-three brief (capabilities and sleeping monitors, ticket #31) is
`docs/design/v1-turn-3.md`. Canvases (`.dc.html` working files) live under
`docs/design/canvases/`.

## Product stance

The app is the editor; the generated scripts are the runtime. A user opens the
window to define layouts, switches from inside it or from the shortcut it generated,
and closes it. Nothing stays resident. Every screen is a workshop, not a dashboard.

## Principles

- **Show hardware truth.** A monitor is Active, Available or Absent, and the UI says
  which, using those words.
- **Never strand the user on a dark screen.** Every step that can leave a monitor
  blank has a visible next action: press the input button, wait, or run again.
- **Every layout is reachable with the app closed.** The shortcut is a first-class
  outcome of saving a layout, and its status is visible on the layout.
- **Copy uses the glossary.** Labels and messages use `CONTEXT.md` terms. A term the
  glossary lacks is added there first.

## v1 screens

1. **Layouts list.** One row per layout with name, on/off summary, Switch, and a
   shortcut indicator. Primary action: Save current layout. First-run empty state.
2. **Save current layout.** Name, plus a live preview of what will be captured:
   monitors on, monitors off, primary. Arrangement is edited in Windows Settings.
3. **Layout detail.** Read-only arrangement summary; monitors in this layout with
   On/Off; ordered steps before and after the apply; audio endpoints; RDP refresh;
   shortcut; fallback on failed apply; collapsed Advanced timing.
4. **Monitors.** Live inventory with alias, reported name, GPU, connector, position,
   size, state (with asleep), DDC-CI support and current input source, and each
   monitor's capabilities (accepted inputs, wake support, modes), read on first
   sight and on Re-check.
5. **Audio.** Endpoints with state and which layouts touch them.
6. **RDP.** Connection files, fingerprint capture, dry run, scheduled task.
7. **Settings.** Default shortcut location, log viewer, config export and import.
8. **Switch progress.** Step list with live status, log tail, result, and the
   "arrange in Settings > Display, then save again" prompt when an apply lands
   monitors somewhere odd.

## Design system

Tokens live in `src/ui/tokens.css` and are the only place a colour, font, radius or
spacing value is written. Components style themselves through tokens.

**Theme rule.** The viewer can be in one of three states: `data-theme="light"`,
`data-theme="dark"`, or unstamped (system). So:

- Bare `:root` declares the complete light palette. Every token exists here.
- `@media (prefers-color-scheme: dark)` redefines only tokens, guarded as
  `:root:not([data-theme="light"])`.
- `:root[data-theme="dark"]` redefines the same tokens again so an explicit choice
  wins in both directions.
- `body` sets its background from a token.

**Palette.** Four to six named colours: a ground, an ink, an accent, and semantic
colours for good, warning and critical that are distinct from the accent. Neutrals
are chosen, biased slightly toward the accent, never defaulted.

**Type.** A display face and a body face with real fallback stacks; a utility face
for data columns with `font-variant-numeric: tabular-nums`. One fixed scale.

**Spacing.** Flex and grid with `gap`; components carry no outer margins. Wide
content scrolls inside its own container.

**Avoid the defaults that read as generated:** Inter or Space Grotesk as the only
face, a purple-to-blue gradient, cream plus serif plus terracotta, everything
centred, one radius on everything, emoji as section markers.

## Copy voice

- Controls say what happens: "Switch to Desk", "Save current layout", "Turn off in
  this layout", "Send HDMI 1 to Ultrawide".
- Errors say what to do next before they say what went wrong.
- Hardware states use the glossary words: Active, Available, Absent.
- No hex codes in the UI. Input sources have names; the code is a tooltip at most.

## Decisions

| Date | Decision | Canvas |
|---|---|---|
| 2026-09-09 | Navigation is master-detail: the sidebar is the layouts list with Save current layout on top and Monitors, Audio, Remote Desktop, Settings pinned at the bottom; the selected layout's detail is always in view | turn 2 |
| 2026-09-09 | Q1: default shortcut location is a Settings choice (Desktop, Start menu, both); each layout shows a shortcut indicator with three states: exists, missing, stale | `v1-settings-window.dc.html` 1a, 1h |
| 2026-09-09 | Q2: monitor state is a chip reading Active (good), Available (warn), Absent (critical), with a one-line note explaining Available and Absent | 1f |
| 2026-09-09 | Q4: input sources are named chips per monitor with the current one highlighted; codes never appear | 1f |
| 2026-09-09 | Q5: arrangement is a read-only schematic beside a spec table; off monitors are chips under the schematic, not rectangles | 1d, turn 2 |
| 2026-09-09 | Q6: elevation is a small dialog stating what needs admin and offering Skip or Continue | 1i |
| 2026-09-09 | Q7: window opens at 1280 by 860, resizable, remembers its size | all |
| 2026-09-09 | Q8: Instrument Sans (display), Libre Franklin (body), JetBrains Mono (data) with Segoe UI fallbacks | tokens |
| 2026-09-09 | Q10: teal accent on cool neutrals; dark set redefines the same tokens | tokens |
| 2026-09-09 | Steps are rows that read as a sentence; a row expands on click to inline selects, with a seconds field only for the timeout wait rule | turn 2 |
| 2026-09-09 | Remote Desktop refresh is a step, not a checkbox | turn 2 |
| 2026-09-09 | Save current layout is a full page, not a dialog | 1c |
| 2026-09-09 | Lists are rows: compact for pick-and-go lists, two-line for editable rows; cards only for the empty state and the switch result | 1a, 1f |
| 2026-09-09 | The sidebar is a shared component (`Sidebar.dc.html`) with props for the selected item, hovered row, empty state and last probe time | turn 2 |
| 2026-09-09 | Switch progress replaces the layout detail in the content area; each step carries a status chip (done, running, waiting, needs you, failed, skipped); Cancel is disabled once the arrangement apply starts | 2c |
| 2026-09-09 | Q3, in-app half: a "needs you" step shows a warning band with the physical action first ("Press the input button on Ultrawide, or turn the console off.") and the seconds remaining | 2c |
| 2026-09-10 | The layout detail header carries a script indicator under the capture time, a dot plus one line: "Script up to date" (good), or "Regenerate the script: ..." (warn) when stale or missing; the header actions for this slice are Open script and Regenerate script, with Open log and Switch arriving with their tickets | 1d, code |
| 2026-09-10 | The schematic also sits above the capture preview on Save current layout, drawn from the monitors the capture will record, so the page shows the picture the layout detail will show after saving; an empty picture says "No monitor is on" | 1c, code |
| 2026-09-10 | "Switch to <name>" is the primary action in the layout detail header, last after Open script and Regenerate script; while another layout is switching it is disabled with "Wait for the switch to <other> to finish." under it, and a refusal (the lock file, a missing script) lands in the detail's band | 1d, code |
| 2026-09-10 | Switch progress keeps the running variant's full-screen layout in every state (Q12 stays open for the cards): "Switching to <name>" with the elapsed time in the data font, one row per step with its chip (waiting, running, done, failed, skipped), the last five log lines, then Cancel, which reads "Cancelling" once sent and carries the line "Cancel is off while the arrangement is being applied." once the apply step reports; the finished states swap the header and the action: "Switched to <name>" with "Applied in N s" (good), "Could not switch to <name>" with a band that puts the next action first and the step and reason under it, "Switch to <name> cancelled", each with "Stopped after N s" and a primary "Back to <name>" | 2c, code |
| 2026-09-10 | The progress screen opens only once the script has started, so a refused switch never flashes it; selecting another layout while a switch runs shows that layout's detail with Switch disabled, and the progress screen returns when the switching layout is selected again; a finished run leaves when Back is pressed or another layout is selected | code |
| 2026-09-10 | The failed state of switch progress keeps the running layout (Q12 stays open for the card) with the band on top: a check failure leads with the physical action the script named and lists the Absent monitors under it; a verify failure leads with "Arrange it in Settings > Display, then Save current layout again." and says "Switch to <name> applied, but <monitor> landed at x,y instead of x,y. layoutswap does not move monitors."; the band carries Open Settings > Display (verify only), Open log and Save diagnostics, the step list is titled "Steps that ran", and the refresh, rotation and scale warnings stay log lines, repeated under the band as "Also noted: ..." on a verify failure | 1i, 2c, code |
| 2026-09-10 | Export config and Import config sit in the sidebar footer where Settings will go, enabled, under the three disabled inventory items; a line beneath them says what the last one did ("Exported to <path>", "Imported N layouts") or why it was refused (critical); both dialogs default to the Desktop, export as layoutswap-config-<date>.json; import keeps this machine's window size (a fresh install takes the file's), removes the folders of the layouts it replaces, selects the first imported layout and drops a finished switch result; import is refused while a switch runs | 1a, code |
| 2026-09-10 | Q3, console half: a shortcut launch prints the needs-you band's two lines once ("Press the input button on <alias>, or turn the other device off." for a send step, "..., or plug it in." for Check monitors, then "Waiting until <alias> is Available") and then the needs-you progress line every ten seconds, which carries the countdown ("92 s left of 120 s"); the app gets that line every second through the fifth progress status, and the log keeps every tenth either way. "The other device" rather than "the console": a second PC shares a monitor the same way | 1i, 2c |
| 2026-09-10 | Steps reorder with Move up and Move down on the expanded row, keyboard-friendly, no drag in the steps slice; the timeline is one list with Check monitors, Apply arrangement and Verify as fixed rows and Add step before and Add step after around the apply; a move at the edge of a side crosses the apply, so the last step before becomes the first step after and back (2026-09-11: the hardware run showed a send step stranded after the apply with no way across) | 2e |
| 2026-09-10 | An edited layout has Save and Discard; Switch is disabled while dirty with "Save the layout first" under it; selecting another layout or opening Save current layout while dirty asks "Save changes to <name>?" in a native dialog with Save, Discard, Keep editing; an after-apply send step on a monitor that is off in the layout carries the note "<alias> is off after the apply, so this step will be skipped" | 1d, 2e |
| 2026-09-10 | An expanded step row starts with a Step select (wait a number of seconds, send an input source to a monitor) that converts the row in place; the send row reads "Send <input> to <monitor> then <wait rule>" as inline selects with the rule's timeout beside it ("up to 5 s"), "(now)" marks the monitor's current input, and "Other code" opens a hex field; the drop wait and the Available wait sit in a collapsed Advanced section under the timeline whose summary reads "Advanced: drop wait 5 s · Available wait 120 s"; Check monitors uses the same Available wait for an Absent monitor | 2e, code |
| 2026-09-10 | The layout detail's monitor rows show each Active monitor's current input source as data text, and the send step's input select marks it; the input list is the fixed table plus "Other code", an unknown code reads "Input 0x1E" | 1d, 2e |
| 2026-09-11 | The layout detail header gains "Re-capture arrangement" as its first action: a fresh probe, then the layout is captured again under its own id with steps, timings and fallback kept; refused while the layout has unsaved edits; the arrangement note reads "Arrange in Windows Settings > Display, then Re-capture arrangement." | 1d, code |
| 2026-09-11 | A send step's drop rule reads "then wait until <monitor> shows <input> or drops": the wait ends when the monitor reports the input over DDC-CI or leaves the PC, since the reference ultrawide never drops; the send row's monitor select marks "(off in this layout)" and the note under an after-apply send to an off monitor ends with "Move it before the apply." | 2e, code |
| 2026-09-11 | "If the switch fails" is a two-radio fieldset under the timeline, Stop and explain (default) or Fall back to Windows Extend when a monitor is not Available; a fallback that lands reads as a failed switch whose band says "Arrange the monitors in Settings > Display, then Save current layout again." and "Windows Extend was applied instead, so every monitor shows the desktop but not in this layout's arrangement.", with Open Settings > Display offered | 1d, 1i, code |
| 2026-09-10 | Save diagnostics opens a native save dialog on the Desktop named layoutswap-diagnostics-<layout folder>-<stamp>.zip; the zip always has five members (app log, switch log, last probe, script, config), a missing one becoming a note; where it went shows as a line under the band; the layout detail header gains Open log between Open script and Regenerate script, and a layout never switched to answers "Switch to the layout once, then open its log" | code |
| 2026-09-09 | Audio inventory rows list which layouts touch each endpoint and how; Remote Desktop shows a match chip per connection file (IDs match, IDs stale) and the scheduled task state with the unlock trigger called out separately | 2a, 2b |
| 2026-09-11 | Asleep is a suffix on the Active chip, ", asleep" at reduced opacity inside the good chip, on Monitors and on the layout detail, where the row's hint reads "Asleep. It is woken before the switch sends its input." | 3a, 3c |
| 2026-09-11 | Monitors rows are the `MonitorRow.dc.html` component: alias in a dashed edit box with a small "edit" label (a bordered input with the focus ring while editing), reported name, a note ("In Desk, Console", "Plugged in, Windows is not drawing to it", "Press its input button, then Refresh"), a data grid of GPU, connector, position, size and the current input source as a chip ("unknown" for a silent panel, "last seen 3840x2160 · 60 Hz" for a monitor that is not Active), then exactly one of: a "Show capabilities" / "Hide capabilities" link with a caret, a muted line ("Switch it on in Windows first" for a monitor that is not Active, "Does not answer over DDC-CI", "Not read yet", or "Reading capabilities" in accent with a spinner), or the expanded panel with Accepts chips in table order and the current one highlighted, Wake as "Can be woken by the app" (good), "Needs a button press to wake" (warn) or "Unknown" (muted), Modes as "3440 x 1440 at 60, 100, 120 Hz" largest first, and "Read on 11 Sep 2026, 11:52"; the header reads "Five monitors connected. Aliases are yours; every other column is read from the hardware." with "Last probe HH:MM", Refresh and Re-check, which reads "Re-checking" with a spinner and is disabled while a read runs; the turn-one input-source chip row and "Send input now" are dropped | 3a, 3b, 3g |
| 2026-09-11 | The send row's input select groups its options under "Accepted by <alias>", then a divider, the rest of the table tagged "not declared" in the data font, a divider and "Other code"; "(now)" stays on the current input as a right-aligned tag; without capabilities the select is unchanged | 3d |
| 2026-09-11 | A send step on an asleep monitor shows a "Wake <alias>" progress row ahead of the send: running with the log line "Waking <alias>" when the monitor can be woken, otherwise needs you with the warning band "Wake <alias>: press a button on it." and "Waiting until <alias> is awake · 92 s left of 120 s"; at the timeout the failed band leads with "Wake <alias>, then switch again." and says "<alias> stayed asleep for N s. These sends did not run: HDMI 1 to Ultrawide, DisplayPort to Side." with the later steps skipped; the failed header keeps "Could not switch to <name>" | 3e, 3f |

## Open design questions

Numbered so a design session can claim one and answer it in the Decisions table.

- **Q9** App icon.
- **Q11** Sidebar row actions (Duplicate, Delete) are small text links under the 44 px target; an icon pair or a right-click menu would fix it.
- **Q12** Switch progress "needs you" and "done" are drawn as cards; the build follows the running variant's full-screen layout, and the cards should be redrawn to match.
- **Q13** Changes found while using the app for real go here, one line each, until a design session picks them up.
  - Refresh rate per layout: investigate listing the rates each monitor offers and letting a layout pick one for each on monitor, applied with the arrangement; today a layout keeps the rate it was captured at and verify only warns on a mismatch. Tracked as a research issue.
  - Settings gains a "Save diagnostics" action under Log, for failures that did not surface as a switch result (the switch result has it since 2026-09-10).
