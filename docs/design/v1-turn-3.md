# Turn 3 brief for Claude Design: monitor capabilities and sleeping monitors

Paste this into the existing project. Turns 1 and 2 live in
`docs/design/canvases/`; keep their tokens, type and components. Terms are from
`CONTEXT.md`: Capabilities, Asleep and Re-check are new there. The spec is issue #28;
this session is ticket #31.

## Decisions from earlier turns that stand

- Monitors (1f) as drawn: compact rows, alias editable inline, state chips Active,
  Available, Absent, input sources as named chips with the current one highlighted,
  Refresh with the last probe time. Q2 and Q4 stay closed.
- Steps as sentence rows that expand to inline selects (2e); "(now)" marks the
  monitor's current input; "Other code" opens a hex field.
- Switch progress (2c) with the needs-you chip and the warning band: physical action
  first, then the countdown line in the data font.
- No hex codes anywhere. Hardware states use Active, Available, Absent.

## What changed since 1f was drawn

- The app reads each Active monitor's **capabilities** when the user presses
  **Re-check**, and keeps them: the inputs it accepts, whether the app can wake it,
  the modes Windows lists for it. The read takes about four seconds per monitor and
  stalls the desktop while it runs, so nothing reads them on its own (the brief
  first said "on first sight"; that was built, felt on the reference machine and
  removed the same day).
- The probe now tells a monitor that is **asleep** from one that is awake. Windows
  still counts it Active, so asleep is a suffix on the state, not a fourth state.
- The "send input now" test action from 1f is out of scope for this slice; drop it.

## Changes to make

### 1f Monitors: capability rows and Re-check

Each row keeps its 1f columns (alias, reported name, GPU, connector, position and
size when Active, state chip, DDC-CI answers or not, current input as a chip) and
gains an expand affordance. Expanded, under the row, three short blocks in the data
font where the values are data:

- **Accepts**: the accepted inputs as named chips in the fixed table's order
  (DisplayPort 1, HDMI 1, HDMI 2 ...), the current one highlighted as in 1f.
- **Wake**: one line, "Can be woken by the app" (good), "Needs a button press to
  wake" (warn), or "Unknown" (muted).
- **Modes**: grouped by resolution, largest first, rates after it: "3440 x 1440 at
  60, 100, 144 Hz". Long lists wrap; no scrolling inside the row.
- A footer line: "Read on 11 Sep 2026, 11:52".

A row that cannot expand shows one muted line instead of the affordance:
"Switch it on in Windows first" (not Active), "Does not answer over DDC-CI" (silent),
"Not read yet" (Active, read pending or never run).

The state chip gains the asleep suffix: "Active, asleep" in the chip, keeping the
Active colour with a muted suffix, so the chip stays one word wide in the common case.

Header: "Monitors", the subtitle with the monitor count, Refresh with the last probe
time as in 1f, and a new **Re-check** button beside Refresh. While a read runs, the
rows being read show "Reading capabilities" as their muted line with the running
treatment from 1g, and Re-check reads "Re-checking" and is disabled.

Alias in edit: as 1f, saved on blur or Enter.

### 1d Layout detail: the asleep suffix

The "Monitors in this layout" rows show "Active, asleep" the same way as on 1f.
Nothing else changes on the detail.

### 2e Send row: accepted inputs first

The input select of an expanded send row, when the monitor has capabilities, lists
the accepted inputs first, a divider, then the rest of the fixed table each marked
"not declared", then Other code. "(now)" still marks the current input. Without
capabilities the select is unchanged. Draw the select open on a monitor that accepts
three inputs.

### 2c Switch progress: the wake band

A send step on a monitor that is asleep goes needs-you before it sends. The band
reads "Wake Ultrawide: press a button on it." with "Waiting until Ultrawide is awake
· 92 s left of 120 s" under it, in the same warning band as the input-button case.
When the app can wake the monitor itself the row simply shows running with the log
line "Waking Ultrawide"; no band. Draw the band state.

The failed state for a wake that timed out leads with "Wake Ultrawide, then switch
again." and names the sends that did not run, as the cancelled result does.

## Screens to add or redraw

- 3a Monitors: five rows in every state (Active expanded, Active asleep, Active
  reading, Available, Absent), one alias in edit, Re-check in the header.
- 3b Monitors, one row that cannot be read for each reason.
- 3c Layout detail with one monitor row reading "Active, asleep".
- 3d Send row expanded with the input select open, grouped.
- 3e Switch progress with the wake band; 3f the failed state after a wake timeout.
- 3g Dark theme of 3a.

## Copy rules

Controls say what happens. Errors say what to do next first. "The app" never
appears in copy; say what happens ("Can be woken", not "The app can wake it") except
where the contrast with a button press is the point. No hex codes.

## Out of scope

Refresh rate per layout (#27), the "send input now" action, a wake step the user
adds, Audio, Remote Desktop, Settings.
