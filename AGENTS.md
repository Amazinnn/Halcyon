# Focus Desktop - Agent Instructions

Focus Desktop: a local focus desktop + agent pet system for Windows (Tauri 2 + Vue 3 + TypeScript + Rust + SQLite). Private remote: https://github.com/Amazinnn/Halcyon.git. Launch: `launch-focus.cmd` (use `launch-focus.cmd rebuild` after Rust/frontend changes).

**Don't start any work before reading `docs/STATUS.md`** - it is the single source of truth for current state, verified items, known issues, and next candidates.

## Repo at a glance

- `apps/desktop` - Tauri 2 + Vue 3 app; windows: desktop / chat / stats / music / pet / grid-overlay / topbar
- `packages/event-schema` - AgentEvent protocol v1 (JSON Schema + TS types + fixtures)
- `docs/` - requirements log, ADRs, architecture, next-phase, STATUS
- `local-focus-desktop-agent-design-v0.2.md` - authoritative design doc (immutable; see Rules)

## Rules (Don't ...)

1. Don't implement a new user requirement before appending it verbatim to `docs/requirements-verbatim.md`; don't edit historical entries (only the status column may be updated later).
2. Don't skip an ADR (`docs/decisions/ADR-00XX.md`) for important architecture decisions; visual-only / small fixes may be recorded in the requirements log only.
3. Don't claim a change is done without running:
   - `cd apps/desktop && npm run build` (vue-tsc + vite)
   - `cd apps/desktop/src-tauri && cargo test --lib`
   - `cd packages/event-schema && npm test`
   Don't hand UI/Rust changes to the user before rebuilding: `launch-focus.cmd rebuild`.
4. Don't commit with non-standard messages; use `feat(...): ...`, `fix(...): ...`, `docs(...): ...`, `chore(...): ...`; don't leave the working tree dirty; don't skip pushing to origin.
5. Don't modify, move, or renumber `local-focus-desktop-agent-design-v0.2.md`; it stays in place with its section numbering (append new sections at the end only if ever needed).
6. Don't add or allow a window to change size without accounting for the size-dependent effects (grid snap, drag/resize glow, layout, typography) - window size affects all of them.

## Environment pitfalls (Don't ...)

- Don't pipe Chinese text through PowerShell to native processes (the us-ascii pipe turns it into `?`); write Chinese files with .NET `WriteAllText` (UTF-8, no BOM).
- Don't retry `git push` through a dead proxy; use the Clash proxy when it is running, otherwise `git -c http.proxy= -c https.proxy= push`.
- Don't fetch new cargo dependencies with proxy env vars set; clear HTTP(S)_PROXY / ALL_PROXY and set `NO_PROXY=crates.io,index.crates.io,static.crates.io,github.com,*.crates.io`.
- Don't assume multi-monitor behavior; this machine is single-monitor (multi-monitor items are N/A).

## Delivery ritual (Don't ...)

- Don't hand over a release without a numbered manual test checklist for the user to verify item by item.
- Don't move on to the next task before the user has reported results.

## Docs navigation

- `docs/STATUS.md` - current state & handoff (read first)
- `docs/next-phase.md` - next candidates / roadmap
- `docs/requirements-verbatim.md` - verbatim user requirements log (#1..#25)
- `docs/decisions/` - ADR-0001..ADR-0011

## Do NOT

- Don't duplicate `docs/STATUS.md` or `README.md` content here; point to them.
- Don't hardcode version numbers here; the current version lives in `README.md` / `docs/STATUS.md`.
- Don't create nested `AGENTS.md` files.
- Don't include secrets or tokens.
