# layoutswap

Windows utility that switches between saved monitor layouts by generating and running
a PowerShell script per layout. Tauri 2 with a Rust backend, Vue 3 renderer,
TypeScript strict, pnpm. Pre-alpha: the definition phase is done and the app is not
yet scaffolded.

## Read before working

- **Vocabulary** lives in `CONTEXT.md`. Identifiers, file names and UI copy use its
  terms; a missing term is added there before naming code after it.
- **Decisions** live in `docs/adr/`. Work that contradicts one says so explicitly,
  naming the ADR, instead of overriding it.
- **Code conventions** live in `CODING_STANDARDS.md`: repository layout, the four
  seams, testing strategy, and the shape of generated scripts.
- **UI decisions and open design questions** live in `DESIGN.md`. Read it before
  any UI work and before starting a Claude Design session; the session brief is
  `docs/design/v1-brief.md`.
- **Process** lives in `CONTRIBUTING.md`: the skill chain from grilling to code
  review, issues, branches, ADR rules.
- **Hardware facts** no code reveals live in `docs/windows-behaviour.md`. Read it
  when touching script generation, timing, monitor identity or audio.
- **Reference output** of the generator lives in `samples/`. Those scripts are what
  the app should produce, not source to refactor; read them when working on
  templates.

## House rules

- Prose in this repo uses commas, colons or full stops where an em-dash might go.
- Personal paths, hostnames and `.rdp` contents stay out of the repo.

## Agent skills

### Issue tracker

Issues are tracked as GitHub Issues on `marioalvarez32/layoutswap` via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage labels are used as-is (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
