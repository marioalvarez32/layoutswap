---
status: accepted
date: 2026-09-09
---

# Generated scripts target Windows PowerShell 5.1

Generated scripts run on Windows PowerShell 5.1, the version installed on every
Windows 10 and 11 machine, rather than PowerShell 7. A user then needs nothing
beyond the app to run a shortcut, and the reference scripts were verified on 5.1.
The cost is a set of language limits the generator must respect: no calls to generic
C# methods, copy-edit-write-back for nested structs, and a one-shot `Add-Type` that
must be guarded. They are listed in `docs/windows-behaviour.md`.

## Considered options

- PowerShell 7: a cleaner language, but users must install it first and every
  hardware fact would need re-verifying.
