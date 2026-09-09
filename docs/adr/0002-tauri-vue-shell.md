---
status: accepted
date: 2026-09-09
---

# Tauri 2 shell with a Vue 3 front end

The desktop shell is Tauri 2 with a Rust backend and a Vue 3 renderer. Rust owns
config files, process spawning and shortcut creation, which are the jobs a switcher
actually has; Vue owns the settings window. Tauri produces a small installer with no
bundled browser, which matters for a utility that runs for a minute and closes. The
cost is two languages and two toolchains in one repo, and shared types that must be
generated rather than written once.

## Considered options

- Electron: familiar, but a hundred-megabyte install for a utility.
- PowerShell module plus a WinForms front end: no second language, but no path to a
  modern UI and no story for a public code sample.
