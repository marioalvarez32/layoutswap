# layoutswap

A Windows utility that switches between saved monitor layouts, including the side
effects a switch needs: monitor input sources, audio endpoints and Remote Desktop
monitor IDs.

## Language

### Layouts

**Layout**:
A named, saved arrangement of monitors plus the steps and side effects that apply
with it.
_Avoid_: profile, preset, mode, configuration

**Capture**:
Taking the arrangement Windows currently shows and saving it as a layout.
_Avoid_: save (as a verb for the whole act), snapshot, record

**Switch**:
Applying a layout, including every step before and after the arrangement changes.
_Avoid_: load, apply (on its own), activate

**Step**:
One ordered action inside a switch, such as sending an input source to a monitor or
waiting for a monitor to become available.
_Avoid_: action, hook, task

**Arrangement**:
Which connected monitors are on and where each one sits, as Windows Settings shows
it. A layout captures one arrangement.
_Avoid_: topology, configuration, setup

**Summary**:
The readable description of a layout's arrangement, derived at capture and never
edited: each monitor's alias, position, size, refresh, rotation, scale and primary.
_Avoid_: preview, details, metadata

**Schematic**:
The read-only picture of an arrangement: one scaled rectangle per on monitor, the
primary marked, off monitors listed as chips beneath it. Drawn from a summary or a
capture preview, never edited.
_Avoid_: diagram, map, preview (that is the capture table), editor

**Verify**:
Reading the arrangement back after a switch and comparing it to the layout's
summary. A switch is applied only when verify passes.
_Avoid_: validate (that is the pre-apply check Windows performs), check, confirm

**On / Off**:
Whether a connected monitor is part of a layout. Off means Windows disconnects the
monitor while it stays plugged in.
_Avoid_: enabled/disabled (reserved for audio endpoints), active/inactive

### Monitors

**Monitor**:
One physical display panel, whether or not Windows is currently using it.
_Avoid_: display, screen, output

**GPU**:
The graphics processor that drives a monitor.
_Avoid_: adapter, display adapter (Adapter is reserved for the code design sense)

**Device path**:
The stable, unique identity of a monitor across reboots.
_Avoid_: monitor ID, device ID, name

**Alias**:
The user's own label for a monitor, shown wherever the monitor appears.
_Avoid_: nickname, label, friendly name (that is the name the monitor reports)

**Primary**:
The monitor Windows places at the origin and puts the taskbar on.
_Avoid_: main, first

**Active**:
Windows is currently drawing to the monitor.
_Avoid_: on (that is the layout property), connected, enabled

**Available**:
The monitor is plugged in and showing the PC's input, whether or not Windows is
drawing to it.
_Avoid_: present, detected, connected

**Absent**:
The monitor is unplugged, powered off, or showing another device's input.
_Avoid_: missing, disconnected, offline

**Input source**:
The physical input a monitor is showing, such as DisplayPort or HDMI 1.
_Avoid_: input, source, channel

### Side effects

**Fingerprint**:
A monitor's position and resolution, used to recognise it for Remote Desktop.
_Avoid_: signature, hash

**RDP profile**:
The record of which monitors a Remote Desktop connection file should span.
_Avoid_: RDP config, connection settings

**Audio endpoint**:
One playback or capture device as Windows lists it, such as a monitor's speakers.
_Avoid_: audio device, sound device, speaker

**Default output**:
The audio endpoint Windows sends sound to unless an app chooses another.
_Avoid_: default device, primary audio

### Generation

**Generated script**:
The runnable file the app writes for a layout so a switch can run with the app
closed.
_Avoid_: layout script, PowerShell file, output

**Shortcut**:
The launcher the app writes for a layout so a switch starts from the desktop or Start
menu.
_Avoid_: link, launcher, lnk

**Probe**:
A read-only generated script that reports the current monitors, input sources and
audio endpoints.
_Avoid_: scan, inventory script, detect

**Stale**:
A generated script or shortcut older than the template or the layout it was made
from. Regenerating clears it.
_Avoid_: outdated, dirty, out of sync

**Lock**:
The file in the app root a running switch script holds, naming its process and its
layout, so one switch runs at a time whether it started from the app or from a
shortcut. The script removes it on exit; the app removes it after a cancel.
_Avoid_: mutex, semaphore, guard

**Diagnostics**:
A single file the app writes on request holding everything needed to debug a
switch: the app log, the layout's switch log, the last probe result, the generated
script and the config.
_Avoid_: debug bundle, support file, crash report
