---
status: accepted
date: 2026-09-09
---

# Generate a PowerShell script per layout and run it

The app renders one PowerShell script per layout from the user's typed config and
runs that script to perform the switch, rather than calling SetDisplayConfig, DDC-CI
and IPolicyConfig from Rust. The hardware logic already exists and is proven on real
monitors (see `samples/`), and a generated script stays inspectable, editable and
runnable from a shortcut with the app closed. The cost is that every Windows
PowerShell 5.1 quirk in `docs/windows-behaviour.md` becomes a rule of the generator
instead of disappearing behind a native crate.

## Considered options

- Rewrite the Win32 calls in Rust with `windows-rs`: cleanest for a portfolio, fully
  unit-testable, but every hardware fact would need re-proving on real monitors.
- Hybrid: Rust orchestrates and small fixed PowerShell helpers make the calls. More
  seams to document for little gain over generation.
