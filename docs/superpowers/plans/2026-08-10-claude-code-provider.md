# Claude Code Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Claude Code as Focus's second real per-pet Agent provider while preserving current Codex behavior, isolation, and workflow result semantics.

**Architecture:** A Claude adapter implements the existing `AgentProvider` contract by starting one `claude -p` stream-json process per turn. `characters.tool` selects the runtime per pet, while a provider-scoped session table preserves separate same-day sessions. The settings-only selector changes that provider and discards only the selected pet runtime; chat and workflow retain their current, provider-neutral UI/event contracts.

**Tech Stack:** Tauri 2, Rust, SQLite/rusqlite migrations, Vue 3/TypeScript/Vitest, Claude Code CLI stream-json.

## Global Constraints

- Focus Demo Pet initially uses the real `claude` provider; Codex remains selectable.
- Focus never reads, writes, or displays Claude credentials, model, proxy, or permission configuration.
- Invoke Claude with its native configuration and permissions; never pass a dangerous skip-permissions option and add no Focus approval UI or tool allowlist.
- Mock remains test-only and no production fallback is added.
- A pet has exactly one selected provider at a time; its Codex and Claude same-day session ids remain separately restorable.
- Direct chat emits normal Agent events; workflow turns suppress raw process events and return exactly one source-labelled final result and target-pet bubble only when `showResult=true`.
- Do not extend the frozen workflow engine or modify `local-focus-desktop-agent-design-v0.2.md`.
- Append the user requirement verbatim, add ADR-0025, update STATUS/next-phase, run build/lib/schema/rebuild verification, commit conventionally, and push `main`.

---

### Task 1: Record the decision and add provider-scoped session persistence

**Files:**
- Modify: `docs/requirements-verbatim.md`
- Create: `docs/decisions/ADR-0025-claude-code-provider.md`
- Modify: `apps/desktop/src-tauri/src/storage.rs`
- Test: `apps/desktop/src-tauri/src/storage.rs`

**Interfaces:**
- Produces `ProviderSessionRow { character_id, provider, session_hash, session_date }` and storage operations that read/write a single pet/provider session.
- Produces a migration which backfills every non-null legacy `characters.current_session_hash` as `provider='codex'` without deleting the legacy columns.
- Later tasks call the storage operations with `AgentProviderKind::as_str()`.

- [ ] **Step 1: Write failing migration and storage regression tests**

Add tests that create a character with a legacy session, apply migrations, and assert a `codex` provider session exists. Add a second test that writes different `codex` and `claude` sessions for the same character/date and reads both unchanged.

- [ ] **Step 2: Run the focused storage tests to verify RED**

Run: `cargo test provider_session --lib` from `apps/desktop/src-tauri`.

Expected: the new provider-session API or migration assertion fails because no provider-scoped table exists.

- [ ] **Step 3: Implement the minimal migration and storage API**

Create `character_provider_sessions` with primary key `(character_id, provider)`, a foreign key to `characters(id)`, `session_hash TEXT NOT NULL`, and `session_date TEXT NOT NULL`. Insert legacy rows with `INSERT OR IGNORE ... SELECT id, 'codex', current_session_hash, session_date FROM characters WHERE current_session_hash IS NOT NULL AND session_date IS NOT NULL`. Implement load/upsert operations only; preserve legacy columns for compatibility during this release.

- [ ] **Step 4: Run the focused storage tests to verify GREEN**

Run: `cargo test provider_session --lib` from `apps/desktop/src-tauri`.

Expected: all provider-session tests pass.

- [ ] **Step 5: Record the user decision before the implementation change**

Append the original request and approved per-pet/provider/session/permission decisions verbatim to the requirements log. Add ADR-0025 defining per-pet provider selection, provider-scoped sessions, native Claude CLI ownership of credentials/permissions, and unchanged workflow result semantics.

- [ ] **Step 6: Commit the task**

Run `git add docs/requirements-verbatim.md docs/decisions/ADR-0025-claude-code-provider.md apps/desktop/src-tauri/src/storage.rs` and commit with `feat(agent): persist provider sessions`.

### Task 2: Implement the real Claude CLI provider and runtime dispatch

**Files:**
- Create: `apps/desktop/src-tauri/src/agents/claude.rs`
- Modify: `apps/desktop/src-tauri/src/agents/mod.rs`
- Modify: `apps/desktop/src-tauri/src/agents/codex.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/workflow.rs`
- Modify: `apps/desktop/src-tauri/src/assets/agent-skills/focus-cli/SKILL.md`
- Test: `apps/desktop/src-tauri/src/agents/claude.rs`
- Test: `apps/desktop/src-tauri/src/lib.rs`
- Test: `apps/desktop/src-tauri/src/workflow.rs`

**Interfaces:**
- Produces `AgentProviderKind::Claude`, `AgentRuntime::Claude`, and a `ClaudeProvider` implementing every `AgentProvider` method used by chat and workflow.
- `ClaudeProvider` starts `claude -p --output-format stream-json --include-partial-messages` for a new turn and adds `--resume <saved-session-id>` for subsequent turns.
- Claude adapter emits the established AgentEvent envelopes and `TurnDone`; all-false display gates all raw events while retaining `TurnDone.result` for workflow.
- Runtime construction uses `characters.tool`; the default character receives `tool='claude'` only when it is Focus Demo Pet.

- [ ] **Step 1: Write failing Claude stream and parameter tests**

Add adapter tests that feed recorded stream-json lines for init, text delta, tool, success, failure, and cancellation. Assert direct display emits schema-valid events, hidden display emits no raw event but retains final result, and command construction uses `-p`, `--output-format stream-json`, `--include-partial-messages`, and `--resume` only with a saved session.

- [ ] **Step 2: Run the Claude-focused tests to verify RED**

Run: `cargo test claude --lib` from `apps/desktop/src-tauri`.

Expected: compile/test failures because the Claude provider/runtime does not exist.

- [ ] **Step 3: Implement the minimal Claude adapter**

Locate `claude.exe`/`claude.cmd` through the inherited Windows PATH, spawn one child per turn with piped stdout/stderr, parse line-delimited stream-json, retain the returned session id, and map text/final/error/cancel information to the established AgentEvent and `TurnDone` contract. Preserve the active-turn guard and interrupt kills only that child. Do not read Claude config or pass permission/model flags.

- [ ] **Step 4: Dispatch providers by pet and connect provider sessions**

Extend runtime cloning/building, `with_agent_for`, chat start/send/resume, workflow agent execution, provider readiness/status, and runtime discard for Claude. Load/save session ids through the provider-session storage API. Update Focus Demo Pet's initial `tool` to `claude`; existing non-demo characters stay codex unless the user changes them.

- [ ] **Step 5: Give Claude the same Focus tool capability**

Install the existing Focus CLI skill into `~/.claude/skills/focus-cli/SKILL.md` without rewriting its guidance, and prepend the Focus release sidecar directory to every Claude child PATH just as Codex receives it. Add a focused test for the install path and PATH ordering.

- [ ] **Step 6: Run focused Rust tests to verify GREEN**

Run: `cargo test claude --lib`; `cargo test provider_session --lib`; and the named workflow result regression tests from `apps/desktop/src-tauri`.

Expected: all focused tests pass, including exactly-once `workflow:agent_result` behavior.

- [ ] **Step 7: Commit the task**

Run `git add apps/desktop/src-tauri/src/agents apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/src/workflow.rs apps/desktop/src-tauri/src/assets/agent-skills/focus-cli/SKILL.md` and commit with `feat(agent): add claude code provider`.

### Task 3: Add the settings-only provider selector

**Files:**
- Modify: `apps/desktop/src/components/SettingsPopover.vue`
- Modify: `apps/desktop/src/stores/agent.ts`
- Modify: `apps/desktop/src/views/chat/ChatView.vue`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Test: `apps/desktop/src/components/SettingsPopover.test.ts`
- Test: `apps/desktop/src/stores/agent.test.ts`

**Interfaces:**
- Produces a per-character `agent_set_provider(characterId, provider)` command accepting only `codex|claude` and dropping only that character's runtime.
- Exposes the selected provider in character/status data for presentation, while chat contains no provider-changing control.

- [ ] **Step 1: Write failing frontend and command tests**

Add a Vitest case showing the Settings Agent row renders a two-value provider selector, invokes the per-character command, refreshes character data, and does not add a selector/button to ChatView. Add a Rust command test rejecting unknown provider and asserting a selected character's runtime is discarded after a valid switch.

- [ ] **Step 2: Run focused tests to verify RED**

Run: `npm test -- --run src/components/SettingsPopover.test.ts src/stores/agent.test.ts` from `apps/desktop` and the named Rust provider-command test from `apps/desktop/src-tauri`.

Expected: tests fail because the selector/command/provider fields do not exist.

- [ ] **Step 3: Implement the minimal settings-only control**

Replace the stale global provider setting/command with the per-character command. Render `Codex` and `Claude` in the existing Agent manager row only. On successful change, refresh characters and status; do not put configuration, model choice, permissions, or provider controls in the chat window.

- [ ] **Step 4: Run focused tests to verify GREEN**

Run the same Vitest and named Rust command tests.

Expected: all focused tests pass and chat remains minimal.

- [ ] **Step 5: Commit the task**

Run `git add apps/desktop/src/components/SettingsPopover.vue apps/desktop/src/stores/agent.ts apps/desktop/src/views/chat/ChatView.vue apps/desktop/src-tauri/src/lib.rs` and commit with `feat(settings): choose agent provider per pet`.

### Task 4: Synchronize status and perform complete automated and real acceptance

**Files:**
- Modify: `docs/STATUS.md`
- Modify: `docs/next-phase.md`
- Test: `apps/desktop/src-tauri/src/agents/claude.rs`
- Test: `apps/desktop/src/components/SettingsPopover.test.ts`

**Interfaces:**
- Documents that Claude Code is now an active M3 provider extension, while Codex's expired-login admission remains a separate issue.
- Delivers a rebuilt release where `focus-cli.exe` is callable by Claude child processes.

- [ ] **Step 1: Write or extend a final regression test for Claude workflow result isolation**

Assert a hidden Claude workflow turn with `showResult=false` yields no chat/bubble result; with `showResult=true`, the engine emits exactly one `workflow:agent_result` and one target-pet bubble.

- [ ] **Step 2: Run the test to verify RED if it exposes an unimplemented path**

Run the named test from `apps/desktop/src-tauri`. If Tasks 1–3 already made it pass, retain it as existing coverage and record that no new production code is justified.

- [ ] **Step 3: Update status documentation**

Update STATUS and next-phase to supersede the Claude freeze, state the per-pet Claude feature and the separate Codex authentication admission block accurately, and list the exact real Claude acceptance scenario.

- [ ] **Step 4: Run complete automated verification**

Run `npm run build` from `apps/desktop`, `cargo test --lib` from `apps/desktop/src-tauri`, and `npm test` from `packages/event-schema`; record each exit status and test count.

- [ ] **Step 5: Rebuild and conduct bounded real Claude acceptance**

Run `launch-focus.cmd rebuild`. In the rebuilt Focus app, select Focus Demo Pet and verify one real Claude reply. Prompt it to use `focus-cli` to create, read, update, run, and delete one uniquely named manual workflow containing exactly one Agent node targeted at Focus Demo Pet with no desktop side effects. Verify a success run record, one source-labelled chat result, one target bubble, list refresh, and deletion. Stop and report the exact provider error if any step fails; never substitute Mock.

- [ ] **Step 6: Commit and push**

Run `git add docs/STATUS.md docs/next-phase.md`, commit with `docs(agent): activate claude code provider`, verify `git status --short` is empty, and push `main` with the repository's reachable proxy route.
