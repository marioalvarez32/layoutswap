---
status: accepted
date: 2026-09-09
---

# A monitor has two identities: device path for layouts, fingerprint for RDP

A layout identifies each monitor by its device path, because friendly names collide
(two identical panels report the same name) and GPU identifiers change every boot.
RDP profiles identify a monitor by its position plus resolution fingerprint instead,
because mstsc numbers monitors by GDI order, which also changes per boot, and the
fingerprint is what stays constant for a given arrangement. The cost is two
identities to explain in the UI and the glossary; the benefit is that both layouts
and `.rdp` files survive reboots, dock changes and driver updates. Background in
`docs/windows-behaviour.md`.
