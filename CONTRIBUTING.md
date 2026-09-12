# Contributing

How work moves through this repo, for humans and for agents. Code conventions are
in `CODING_STANDARDS.md`; UI decisions are in `DESIGN.md`; this file is the process.

## Prerequisites

- Claude Code with the `mattpocock-skills` plugin. The per-repo setup that plugin
  needs (`docs/agents/`) is already committed.
- Node with pnpm, and the Rust toolchain, once the scaffold lands.

## The flow

Every change follows the same chain. Each step is a skill you invoke.

1. `/grill-with-docs`: interview about the change until the design is sharp. Terms
   land in `CONTEXT.md` and decisions in `docs/adr/` as they crystallise.
2. `/to-spec`: turn that conversation into a spec and publish it as a GitHub issue.
3. `/to-tickets`: break the spec into vertical-slice tickets with blocking edges,
   also as GitHub issues.
4. `/implement`: build one ticket. It drives `/tdd` at the seams the spec named,
   runs the full suite, then `/code-review`, then commits.
5. `/code-review`: two axes, standards (against `CODING_STANDARDS.md`) and spec.

Steps 1 to 3 stay in one context window. Each `/implement` starts a fresh session
from its ticket.

**On-ramps** that feed the flow:

- `/triage` for issues you did not write. Tickets from `/to-tickets` are already
  agent-ready and skip triage.
- `/diagnosing-bugs` when something is broken. Its post-mortem may hand off to
  `/improve-codebase-architecture` when there was no seam to lock the bug down.
- `/wayfinder` for an effort too foggy for one spec; it plans, then merges onto the
  flow at `/to-spec`.
- `/improve-codebase-architecture` every few days once code exists. It surveys;
  a finding becomes an idea that enters at step 1.

## Issues

Specs and tickets live in GitHub Issues on this repo. Conventions for creating,
reading and labelling them are in `docs/agents/issue-tracker.md`; the label
vocabulary is in `docs/agents/triage-labels.md`.

## Design work

1. Read `DESIGN.md`, then the brief it points at.
2. Run the `design` skill. Claim one open question or one screen per session.
3. Keep the `.dc.html` working files under `docs/design/canvases/`.
4. Record the outcome as a row in the Decisions table of `DESIGN.md`, and remove
   the open question it answered.

## Decision records

Write an ADR only when the decision is hard to reverse, would surprise a reader
without context, and came from a real trade-off. Files live in `docs/adr/` as
`NNNN-slug.md`, numbered from the highest existing number plus one. The whole
template is a title and one to three sentences; add Considered options or
Consequences only when they carry information. Refer to one in prose as `ADR-0003`.

## Branches and commits

- `main` is releasable. Work happens on `feat/<slug>` or `fix/<slug>` branches.
- Commit subjects follow Conventional Commits: `feat(layouts): capture on/off set`.
  A `commit-msg` hook runs commitlint (`commitlint.config.mjs`) and refuses a
  subject that does not fit, because semantic-release reads the subjects to pick
  the version and write the changelog. `pnpm install` sets the hook up.
- A pull request references its issue and passes lint, typecheck and tests.

## Releases

Releases are automatic. After every green CI run on `main`, the Release workflow
runs semantic-release (`.releaserc.json`): a `fix` commit since the last release
makes a patch version, a `feat` a minor, a `BREAKING CHANGE` footer a major; with
none of those, nothing happens. It writes the notes into `CHANGELOG.md`, bumps the
version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`
(`scripts/set-version.mjs`), builds the installers, commits and tags, and publishes
a GitHub release with the installers attached. Nothing in `CHANGELOG.md` is edited
by hand. The `v0.0.0` tag on the first commit is what makes the first release
`0.1.0`.

Before merging work that will release, run the hardware checklist on a real
multi-monitor machine:

1. The probe lists every connected monitor with the right state.
2. Capture a layout, switch away, switch back; the arrangement is identical.
3. A layout with an input-source step hands the monitor to the other device and
   the return switch brings it back.
4. Audio endpoints and RDP files match what each layout declares.

## Code of conduct

This is a solo portfolio project and does not carry a code of conduct. Issues and
pull requests from others are welcome and will be triaged.
