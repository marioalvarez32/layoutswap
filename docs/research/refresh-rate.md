# Research: refresh rate per layout

Answers issue #27. Findings from a validate-only run of
`SetDisplayConfig` on the reference machine on 2026-09-11 (four active monitors,
nothing applied). The script
passed `SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_VALIDATE`, with and without
`SDC_ALLOW_CHANGES`, over clones of the active paths and modes from
`QueryDisplayConfig`, editing one path target's `refreshRate` on the Acer (1920x1080,
running at 200 Hz, offering 50, 59, 60, 119, 120, 180 and 200 by
`EnumDisplaySettingsEx`).

## What validate said

| Edit to the Acer's path | With `SDC_ALLOW_CHANGES` | Without |
|---|---|---|
| Unchanged | 0 | 0 |
| 50/1, target mode index kept | 0 | 0 |
| 50/1, target mode index invalid | 0 | 0 |
| 999/1, target mode index kept | 0 | 0 |
| 999/1, target mode index invalid | 0 | 1610 |
| 50/1, target mode info rewritten too | 0 | 0 |
| 999/1, target mode info rewritten too | 0 | 1610 |
| 0/0, target mode index invalid | 87 | 87 |
| 144/1 (not offered), index invalid | 0 | 1610 |
| 144/1 (not offered), index kept | 0 | 0 |
| 120000/1001, index invalid | 0 | 1610 |
| 119/1, index invalid | 0 | 1610 |
| 59/1, index invalid | 0 | 1610 |
| 60000/1001, index invalid | 0 | 0 |
| 200000/1000 (current), index invalid | 0 | 0 |

## What it means

- **`SDC_ALLOW_CHANGES` makes validate say yes to anything.** Even 999 Hz passes. The
  switch template applies with that flag, so it cannot tell a bad rate from a good
  one; Windows substitutes a mode on apply. A refresh-rate apply must drop the flag,
  or validate without it first.
- **A kept target mode index wins over the path's refresh rate.** With the index
  pointing at the current target mode, the `refreshRate` field is ignored and any
  value passes. To ask for a rate, set the path target's `modeInfoIdx` to
  `DISPLAYCONFIG_PATH_MODE_IDX_INVALID` and put the rate in `refreshRate`; the source
  mode (resolution, position) stays as captured.
- **The rate must be the monitor's exact rational.** 60000/1001 (59.94 Hz) passes
  where 59/1 fails, and 50/1 passes as an integer; 119/1 and 120000/1001 both fail
  although `EnumDisplaySettingsEx` lists 119 and 120, so those modes carry some other
  rational. The integer `dmDisplayFrequency` is a rounded label, not the value Windows
  matches on, and no documented call lists the exact rationals of the modes a target
  offers. The current mode's rational is in the active target mode's
  `vSyncFreq` (the ultrawide runs 59973/1000, which reads 60 in the list).
- **Rewriting the target mode info is unnecessary.** Invalidating the index and
  setting the rate is enough; rewriting `vSyncFreq` and `pixelRate` on the mode
  changed nothing.

## What a refresh-rate slice would do

1. Read the rates each monitor offers with `EnumDisplaySettingsEx` (integer labels)
   for the pick list, and validate the layout's chosen rate as candidates before
   trusting it: n/1, n×1000/1001, and the captured mode's own rational, each through
   `SetDisplayConfig` with `SDC_VALIDATE` and no `SDC_ALLOW_CHANGES`; the first that
   returns 0 is the rational to store with the layout.
2. Apply with the target mode index invalid and the stored rational on the path,
   without `SDC_ALLOW_CHANGES` so a rate Windows cannot honour fails as 1610 instead
   of silently landing elsewhere; keep today's flags for layouts that pick no rate.
3. Verify already reads the rate back; a chosen rate would turn the refresh warning
   into a failure for that monitor.

## Caveats

- A background run of the same script reported that `SDC_VALIDATE` starts returning
  errors after roughly 112 calls in one process and that the baseline validate
  needed time to return 0 again. That run's transcript was lost, so the figure is
  unconfirmed; keep any candidate search under a hundred calls per process and spread
  across runs.
- The test edited one monitor's rate at a time. Two monitors changing rate in one
  apply, and a rate change together with a resolution change, were not validated.
- Validate is not apply. A real apply of a chosen rate is a hardware test for the
  slice that ships it.

The script is `validate-refresh.ps1` from the 2026-09-11 session scratchpad; it hard
guards the flags so only validate can reach `SetDisplayConfig`.
