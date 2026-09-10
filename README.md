# layoutswap

Switch between saved monitor layouts on Windows with one click, including the side
effects a real switch needs: hand a monitor's input to another device, turn monitors
on or off, keep Remote Desktop monitor IDs correct, and enable or disable audio
endpoints. Every layout becomes a generated script and a shortcut, so switching
works with the app closed.

**Status:** pre-alpha. The app opens, reads the connected monitors, and captures the
arrangement Windows shows as a named layout with its summary. Switching is the next
slice. The reference scripts under `samples/` do the job today by hand.

## Why

One ultrawide monitor is shared between a PC and a console. Switching it means
changing the monitor's input, letting Windows drop the monitor, lighting a spare one
in its place, and then undoing all of that later, while Windows reshuffles Remote
Desktop monitor numbers and re-enables monitor speakers every time the topology
changes. Two PowerShell scripts grew to handle this on one machine. layoutswap
turns them into something anyone can configure.

## What v1 does

- Unlimited named layouts, captured from the arrangement Windows currently shows.
- Per layout: which connected monitors are on or off; ordered steps before and
  after the apply, such as sending an input source to a monitor and waiting for it;
  enable, disable or set the default audio endpoint; refresh Remote Desktop
  profiles.
- A Switch button per layout in the app, plus a generated script and shortcut per
  layout for switching with the app closed.
- A live inventory of monitors with their state: Active, Available or Absent.
- A scheduled task that refreshes Remote Desktop monitor IDs at logon and unlock.

## What v1 does not do

- A draggable arrangement editor. Arrange in Windows Settings, then save.
- A tray icon, hotkeys, or any resident process.
- Anything outside Windows.

**Open decision:** which actions need elevation and how the app asks for it.

## Architecture at a glance

The app is the editor; generated scripts are the runtime. Rust renders a PowerShell
script per layout from typed config and runs it; the Vue window never handles script
text. Layouts identify monitors by device path so they survive reboots. The
reasoning is in `docs/adr/`, starting with ADR-0001, ADR-0002 and ADR-0003, and the
hardware facts behind them are in `docs/windows-behaviour.md`.

| Part | Choice |
|---|---|
| Shell | Tauri 2, Rust backend |
| UI | Vue 3, TypeScript strict, Pinia |
| Generated scripts | Windows PowerShell 5.1 |
| Tooling | pnpm, Vite, Vitest, ESLint |

## Hardware it was built against

| Part | Model |
|---|---|
| Laptop | Lenovo Legion 5 15IAX10, NVIDIA RTX 5070 + Intel Arc |
| Dock | Kensington AD4010T4 |
| Monitors | ASUS VG34VQEL1A ultrawide, two MSI MP165 E6, Acer KG241Y X1, LG FULL HD, built-in panel |

## Development

Prerequisites: Node with pnpm, the Rust toolchain (`x86_64-pc-windows-msvc`), the
Visual Studio C++ build tools, and the WebView2 runtime that ships with Windows 11.

```
pnpm install          # once per clone
pnpm tauri dev        # opens the app with hot reload
pnpm lint             # ESLint: lint and formatting check
pnpm format           # ESLint --fix: rewrite formatting
pnpm typecheck        # vue-tsc, TypeScript strict
pnpm test             # Vitest with Vue Test Utils
pnpm test:rust        # cargo test for the backend
pnpm lint:rust        # cargo clippy with warnings denied
pnpm generate:types   # regenerate src/domain/generated/types.ts from the Rust types
```

`cargo test` also regenerates the TypeScript bindings, so a change to a shared Rust
type shows up as a diff in the generated file. The app writes everything it owns under
`%LOCALAPPDATA%\layoutswap`; delete that folder to start fresh.

The engineering process, from idea to merged change, is in `CONTRIBUTING.md`. Code
conventions are in `CODING_STANDARDS.md`; UI decisions in `DESIGN.md`; the domain
vocabulary in `CONTEXT.md`.

## License

MIT. See `LICENSE`.
