---
status: accepted
date: 2026-09-09
---

# A layout stores the raw display-config blob plus a derived summary

A layout holds two things: the SetDisplayConfig path and mode arrays exactly as
Windows returned them at capture, base64-encoded with the GPU device paths needed
to remap identifiers at apply time, and a readable summary derived from the same
probe. The blob is what gets applied, because copying those structs verbatim is the
approach proven on real hardware (see `samples/Switch-Layout.ps1`); the summary is
what the UI shows and what verify compares against. The summary is never edited,
so the two cannot drift. The cost is that any future in-app edit to an arrangement
must rewrite the blob, which is the arrangement editor's problem, not v1's.

## Considered options

- Blob only: the UI would have nothing to display without re-probing.
- Fully decoded model rebuilt into structs by the script: every field Windows
  filled in would need re-deriving, which is where the hardware surprises live.
