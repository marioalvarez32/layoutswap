# Turn 2 brief for Claude Design: layoutswap v1 settings window

Paste this into the existing project. Turn 1 lives in
`docs/design/canvases/v1-settings-window.dc.html`; keep its tokens, type and
components. Terms are from `CONTEXT.md`.

## Decisions from turn 1 that stand

- Fonts: Instrument Sans, Libre Franklin, JetBrains Mono for data. Q8 closed.
- Palette: teal accent on cool neutrals, dark set from the same token names. Q10 closed.
- Arrangement: the schematic variant (1d). Q5 closed. Drop 1e.
- Monitor state chips Active, Available, Absent as drawn in 1f. Q2 closed.
- Input sources as named chips per monitor, code never shown. Q4 closed.
- Elevation dialog as drawn in 1i. Q6 closed.
- Window 1280 by 860, resizable, remembers its size. Q7 closed.
- Save current layout stays a full page, not a dialog.
- Screens 1f Monitors, 1g probing, 1h Settings, 1i failure and elevation are approved as drawn.

## Changes to make

### Navigation: the sidebar is the layouts list

Replace the five equal sidebar items. New sidebar, 212 px:

- Top: a full-width **Save current layout** button.
- Then one compact row per layout: name, and a small data-font count "3 on · 1 off". The selected layout uses the current accent-soft treatment. Row actions (Duplicate, Delete) appear on hover and focus only.
- Bottom, pinned: **Monitors**, **Audio**, **Remote Desktop**, **Settings** as compact items, separated from the layouts by a rule. Keep "Last probe" under them.
- Content area: the selected layout's detail, always in view. Remove the "← Layouts" back link everywhere. The sticky header holds the name, the shortcut indicator, and the action buttons as in 1d.

Redraw 1a as the merged layouts-plus-detail screen with four layouts. Redraw 1b so the empty state has an empty sidebar (Save button, pinned items) and the current empty-state copy in the content area.

### Steps: inline sentence that expands on click

A step is one of three kinds. Collapsed, a row reads as in 1d: drag handle, number, sentence, wait chip, delete. Expanded (click the row or the chip), the sentence becomes inline controls that still read left to right:

- Send input: `Send [HDMI 1 ▾] to [Ultrawide ▾]` then a wait rule select with three values: no wait · until [Ultrawide] drops · until [Ultrawide] is Available, up to [120] s. The seconds field appears only for the third value.
- Wait: `Wait [3] seconds`.
- Refresh RDP profiles: no options on the row; retries live in Advanced.

Selects use native `<select>` styled with the tokens, 34 px tall inside the row, 44 px hit target on the row itself. Draw one expanded row of each kind.

### Remote Desktop refresh is a step only

Remove the Remote Desktop checkbox section from the layout detail. Step 4 stays. Move the Shortcut section into the space it frees, beside Audio.

### Arrangement schematic

An off monitor has no position, so it is not a rectangle in the schematic. Remove the dashed "Off" box; add a chip row under the schematic: "Off in this layout: Portrait, TV". Keep the spec table beside the schematic.

### Audio: a way to set the default output

Add a radio to each **Playback** endpoint labelled "Default output". It is enabled only when that endpoint's choice is Enable. Capture endpoints have no radio. Draw one endpoint with the radio selected, one enabled but not selected, one disabled because the endpoint is set to Disable.

## Screens to add

### 2a Audio inventory

Sidebar with Audio selected. Header: "Audio", subtitle with the endpoint count, Refresh with last probe time. Compact rows grouped Playback then Capture: name, state chip (Active, Disabled, Absent), and a muted line listing the layouts that touch it ("Console: disable · Desk: enable, default"). An endpoint no layout touches reads "Not used by any layout". Include one endpoint that is Absent because its monitor is off.

### 2b Remote Desktop

Sidebar with Remote Desktop selected. Two blocks:

- **Connection files**: compact rows, one per `.rdp` file: file name, the monitors it spans by alias in capture order, and a match chip: "IDs match" (good) or "IDs stale" (warn). Row actions on hover: Recapture, Remove. Header actions: Add file, Capture fingerprints, Dry run.
- **Scheduled task**: a card-free block. Line one: "Installed · logon" or "Not installed". Line two: "Unlock trigger: not installed, needs administrator" with an Install button that leads to the 1i elevation dialog. Buttons: Install, Remove.

Draw two states: task not installed, and task installed with the unlock trigger missing.

### 2c Switch progress, running

Shown in the content area after pressing Switch, replacing the detail until done. Header: "Switching to Console", elapsed time in the data font. Then the step list with a status chip per step: done, running, waiting, needs you, failed, skipped. Below it a log tail, five lines, data font. One button: Cancel, which is disabled once the arrangement apply has started.

Draw three states:

- Running: step 1 done, step 2 running with the elapsed seconds.
- Needs you: step 2 waiting up to 120 s for Ultrawide to become Available, with the copy "Press the input button on Ultrawide, or turn the console off." and the seconds remaining.
- Done: all steps done, "Applied in 11.4 s", one button "Back to Console".

### 2d Dark theme

One artboard of the merged layouts-plus-detail screen (new 1a) with `data-theme="dark"`, to prove the dark token set on the busiest screen.

## Variants list

Draw each of these as its own artboard or a clearly separated state within one:

- 1a: four layouts; one layout selected.
- 1b: empty state.
- 1c: Save current layout, as drawn.
- 1d: layout detail with the step rows collapsed; a second copy with one row of each kind expanded.
- 1f: Monitors, as drawn.
- 1g: Monitors probing, as drawn.
- 1h: Settings, as drawn.
- 1i: failure result and elevation dialog, as drawn.
- 2a: Audio inventory.
- 2b: Remote Desktop, not installed; installed with unlock trigger missing.
- 2c: Switch progress: running; needs you; done.
- 2d: dark theme of 1a.

## Copy rules

Controls say what happens. Errors say what to do next first. Hardware states use Active, Available, Absent. No hex codes anywhere.

## Out of scope

Draggable arrangement editor, tray icon, hotkeys, app icon (Q9 stays open for a later session).
