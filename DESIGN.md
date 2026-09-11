# Design

The standing record of UI decisions. Claude Design reads the repo's tokens and this
file before drawing; it never writes a record of its own, so every design session
ends by adding a row to the Decisions table below. The brief for the current design
effort is `docs/design/v1-brief.md`; the turn-two brief is `docs/design/v1-turn-2.md`. Canvases (`.dc.html` working files) live under
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
   size, state, DDC-CI support and current input source.
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
| 2026-09-10 | Save diagnostics opens a native save dialog on the Desktop named layoutswap-diagnostics-<layout folder>-<stamp>.zip; the zip always has five members (app log, switch log, last probe, script, config), a missing one becoming a note; where it went shows as a line under the band; the layout detail header gains Open log between Open script and Regenerate script, and a layout never switched to answers "Switch to the layout once, then open its log" | code |
| 2026-09-09 | Audio inventory rows list which layouts touch each endpoint and how; Remote Desktop shows a match chip per connection file (IDs match, IDs stale) and the scheduled task state with the unlock trigger called out separately | 2a, 2b |

## Open design questions

Numbered so a design session can claim one and answer it in the Decisions table.

- **Q3** Console-window half only: the plain-text progress a shortcut launch prints while a monitor is dark. The in-app half is decided (2c); the console copy in 1i is a draft to verify on hardware.
- **Q9** App icon.
- **Q11** Sidebar row actions (Duplicate, Delete) are small text links under the 44 px target; an icon pair or a right-click menu would fix it.
- **Q12** Switch progress "needs you" and "done" are drawn as cards; the build follows the running variant's full-screen layout, and the cards should be redrawn to match.
- **Q13** Changes found while using the app for real go here, one line each, until a design session picks them up.
  - Settings gains a "Save diagnostics" action under Log, for failures that did not surface as a switch result (the switch result has it since 2026-09-10).
