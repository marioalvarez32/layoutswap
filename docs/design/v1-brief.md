# Design brief: layoutswap v1 settings window

Start here for a Claude Design session. `DESIGN.md` holds the standing rules and
the open questions; this brief holds what to draw and every option each screen
exposes. Terms are from `CONTEXT.md`.

## What to design

The v1 settings window for a Windows 11 desktop app. Static mockups first, one
artboard per screen, in light and dark. No tray, no resident process: the window
opens, the user works, the window closes.

## Who uses it

One person with several monitors and at least one other device (a console, a
second PC) sharing one monitor's input. Comfortable with Windows display settings.
Not interested in hex codes or GPU identifiers.

## Screens and their options

### Layouts list

- One row or card per layout: name, count of monitors on, count off, Switch button,
  shortcut indicator (exists, missing, stale).
- Primary action: **Save current layout**.
- Secondary: open a layout, duplicate, delete.
- Empty state on first run: what a layout is in one sentence, then the primary action.

### Save current layout

- Name field.
- Live preview of what will be captured: monitors on with alias, position and size;
  monitors off; the primary.
- One line noting that arrangement is edited in Windows Settings > Display before
  saving.
- Checkbox: create a shortcut now (default on).

### Layout detail

- **Arrangement summary**, read-only: each monitor with alias, position, size,
  refresh, rotation, scale, primary marker. A small schematic is optional (Q5).
- **Monitors in this layout**: every connected monitor with On or Off. Off is a
  toggle. On, for a monitor currently Off, shows the hint "Turn it on in Windows
  Settings, then Save current layout again."
- **Steps**: an ordered list, split into before the apply and after it. Each step
  is one of: send an input source to a monitor (monitor by alias, input source by
  name, wait rule: none, wait until the monitor drops, wait until it is Available
  with a timeout); wait N seconds; refresh RDP profiles. Add, reorder, remove.
- **Audio**: each endpoint with three choices: enable, disable, leave alone. One
  endpoint may be marked default output.
- **RDP**: refresh profiles after this switch, on or off.
- **Shortcut**: exists or not, location, keep the console window open after the run.
- **Fallback when the apply fails**: fall back to Windows Extend, or stop and explain.
- **Advanced** (collapsed): drop wait (default 5 s), Available wait (default 120 s),
  retry count and spacing for the RDP refresh (defaults 7 and 15 s).
- Actions: Switch to this layout, Save, Regenerate script, Open script, Open log.

### Monitors

- Live list: alias (editable inline), reported name, GPU, connector, position, size,
  state (Active, Available, Absent), DDC-CI supported or not, current input source.
- Per monitor: the named input list it supports (DisplayPort, HDMI 1, HDMI 2, and
  so on), and a "send input now" test action.
- Refresh action, with the time of the last probe.

### Audio

- Endpoints with flow (playback or capture), name, state, and the layouts that
  touch each one.
- Refresh action.

### RDP

- Connection files the app knows about; add and remove.
- Per file: the monitors it spans, by alias, and whether the current IDs match.
- Capture fingerprints from currently-correct files.
- Dry run: show what would change without writing.
- Scheduled task: installed or not, triggers (logon; unlock needs admin), install
  and remove.

### Settings

- Default shortcut location (Desktop, Start menu, both).
- Log viewer with the path and a clear action.
- Config export and import.
- About: version, licence, link to the repo.

### Switch progress and result

- Step list with live status per step: waiting, running, done, needs you, failed.
- Log tail.
- Result: applied, or what to do next.
- When an apply lands a monitor somewhere odd: "Arrange it in Settings > Display,
  then Save current layout again."
- From a shortcut launch this runs in a console window, so the copy must read
  well as plain text too.

## States to draw

Empty, loading, monitor Available but not Active, monitor Absent, DDC-CI
unsupported, apply failed, elevation needed, shortcut missing.

## Constraints

- Windows 11 desktop window; default size is Q7.
- 44 px hit targets; everything reachable by keyboard; visible focus.
- Light and dark from the same tokens, per the theme rule in `DESIGN.md`.
- No tray or notification affordances.

## Copy starters

Switch to NAME · Save current layout · Turn off in this layout · Send INPUT to ALIAS
· Wait until ALIAS is Available · Refresh RDP profiles · Create shortcut ·
Regenerate script · Open log · ALIAS is Absent: press its input button, then try
again.

## Open questions

Q1 to Q10 in `DESIGN.md`. Claim one per session and record the answer there.

## Out of scope for v1

Draggable arrangement editor, tray icon, global hotkeys, per-user profiles,
anything not Windows.
