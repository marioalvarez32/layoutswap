---
status: accepted
date: 2026-09-09
---

# Rust renders every script; the webview never sends script text

The Vue renderer sends typed layout and config values across the Tauri command seam.
Rust turns them into script text and runs it. Script text never crosses the seam in
either direction. A compromised or buggy webview therefore cannot run arbitrary
PowerShell, and the 5.1 quirks live in exactly one module (`script::render`). The
cost is that golden-file tests for generated scripts live in Rust rather than as
Vitest snapshots, and the renderer must be built before the UI can preview a script.

## Consequences

- Shared types are defined in Rust and generated for TypeScript.
- A "preview generated script" feature is a read-only command that returns text,
  never accepts it.
