# Windows behaviour the app must respect

Facts learned on real hardware while building the reference scripts under `samples/`.
None of them is visible from code or documentation alone. Every generator rule and
most timing defaults trace back to one of these.

## Displays

- **A monitor drops off the PC when its input changes.** After a DDC-CI command sends
  a monitor to another input (VCP code 0x60), Windows may keep the DisplayPort link
  idle. The PC cannot always pull the monitor back on its own. Switching back needs
  the monitor to become Available again, which happens when the user presses the
  monitor's input button or the monitor has auto source detection enabled.
- **Some monitors keep the link alive while showing another input.** The reference
  ultrawide stays Available to Windows while displaying the console. The layout apply
  is what turns it off; the wait after the DDC command is a short grace period, not
  a guarantee.
- **A discrete GPU caps active displays.** The reference laptop's discrete GPU drives
  at most four active displays. With five connected, one is always off. A layout
  therefore records which connected monitors are on and which are off, and applying
  it activates only the on set.
- **Hybrid graphics mode changes the topology.** Toggling hybrid mode in the vendor
  tool changed which GPU owned which output and invalidated saved layouts. The
  reference layouts were re-saved with hybrid mode off.
- **GPU identifiers change on every boot.** The LUID that SetDisplayConfig uses for a
  GPU is not stable. Saved layouts store the GPU device path instead and remap it to
  the current LUID at apply time.
- **Friendly names collide.** Two identical panels report the same EDID name. The
  device path is the only stable, unique identity for a monitor.
- **A monitor answers DDC-CI only while Windows draws to it.** The physical monitor
  handle comes from the GDI name, which exists only for an Active monitor. So an input
  source is sent before the apply when the monitor is leaving and after it when the
  monitor is coming back. The reference scripts settle 3 s after a monitor becomes
  Available before applying, and 2 s after the apply before sending an input; the
  generated script keeps the 2 s after every real apply, before the after-steps and
  verify.
- **DDC-CI reads are slow and some monitors stay silent.** Reading the input source
  (VCP code 0x60) takes tens to hundreds of milliseconds per monitor over dxva2, and a
  built-in panel or a monitor behind some docks never answers. The probe starts every
  active monitor's read at once and waits 1.5 s for all of them together, reporting
  "not answering" for the rest; a read that never returns keeps its handle until the
  probe exits. In clone mode one GDI name covers several physical monitors, which
  cannot be told apart, so it reads as not answering too.
- **Windows rejects an arrangement with a floating monitor.** Every active monitor
  must share an edge with the group that contains the primary.
- **Apply returns numeric codes.** Error 87 usually means a monitor in the layout is
  not connected; 1610 means a bad configuration; 31 is a general failure.

## Audio

- **Display audio endpoints get a new GUID on every topology change.** Disabling one
  in Sound settings disables only that GUID. After the next layout change the monitor
  speakers reappear as a new, enabled endpoint. The reference script re-disables any
  active endpoint whose name matches a list, on every run.
- **The Sound control panel's own interface works without elevation.** The
  undocumented but stable IPolicyConfig COM interface enables, disables, and sets
  the default endpoint from a normal user session.

## Remote Desktop

- **mstsc monitor IDs change after a reboot**, especially with an integrated plus a
  discrete GPU. The ID is the trailing number of the GDI device name minus one.
- **Position plus resolution is stable across reboots.** Fingerprinting each monitor
  that way, then recomputing the IDs, is what keeps `.rdp` files correct.
- **The first ID in `selectedmonitors` is the session's primary.** Order is preserved
  from capture, never sorted.
- **`.rdp` files are UTF-16 LE.** Read and write them with that encoding.
- **Docks enumerate slowly.** At logon the monitors may not all be present for a
  minute or more. The reference task retries seven times, fifteen seconds apart.

## Scheduled tasks

- **Docking flips the power source for a moment.** With default task settings that
  kills the task instantly (exit 0xC000013A). Tasks must allow start on battery and
  must not stop when switching to battery.
- **The session-unlock trigger needs an elevated shell to register.** Registering
  from a normal session throws an access-denied error. The reference script falls
  back to a logon-only trigger and tells the user to re-run elevated.

## Windows PowerShell 5.1

- **Nested struct assignment silently does nothing.** `$path.source.id = 1` edits a
  copy. Copy the struct out, edit it, and write it back.
- **Generic C# methods cannot be called.** Expose non-generic typed wrappers from
  compiled C# instead.
- **Add-Type is one-shot per session.** Guard the type definition with a
  `PSTypeName` check so re-running in the same session does not fail.
- **`Write-Host` can be shadowed** to tee every line into a log file, which is how
  a run launched from a shortcut stays inspectable afterwards.

## Hardware the reference scripts were verified on

| Part | Model |
|---|---|
| Laptop | Lenovo Legion 5 15IAX10, hybrid graphics off |
| GPUs | NVIDIA RTX 5070 (driver 616.64), Intel Arc |
| Dock | Kensington AD4010T4 (Thunderbolt 4) |
| Ultrawide | ASUS VG34VQEL1A, 3440x1440, DisplayPort to the PC, HDMI 1 to the console |
| Portable | MSI MP165 E6, 1920x1080 (two units) |
| Desk monitors | Acer KG241Y X1, LG FULL HD, both 1920x1080 |
| Built-in | 2560x1600 |
