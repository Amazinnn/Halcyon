# ADR-0028: Visible Skill Invocation as User Input

- Status: Accepted (2026-08-11)
- Supersedes: ADR-0026 decision 5 only
- Related: requirements #86, ADR-0022, ADR-0024, ADR-0025

## Context

The chat composer previously sent a separate `skillName` field to the Tauri
boundary. Focus then read `SKILL.md` and injected its contents into the
provider prompt. That made a selected Skill invisible in the user's message,
made deletion behave like ordinary text editing, and gave the Focus backend a
second prompt-construction responsibility.

## Decision

1. Skill discovery remains provider-scoped through `agent_list_skills`.
2. Selecting a Skill creates one visible composer atom. The atom is rendered
   as `$skill-name` with surrounding space and is removed atomically by
   Backspace/Delete at the input boundary.
3. Sending composes the direct user message as `$skill-name  text` (or `text`
   without a Skill). This exact string is persisted in visible history and is
   the only `text` sent to `agent_start_thread` or `agent_send`.
4. Focus does not read, validate, or inject selected `SKILL.md` content for
   chat. The real Provider receives the same user input that the user sees.
5. The chat author label uses the selected pet's display name. Provider
   selection remains outside chat, in Agent settings.

## Consequences

The UI and Provider boundary have one source of truth for a Skill invocation.
Skill instructions must be available to the Provider through its normal local
skills configuration or the user's explicit input; Focus does not add a second
prompt layer. Provider session, streaming, workflow result, and interruption
semantics are unchanged.

## Verification

`chat-composer.test.ts`, agent store tests, ChatView source tests, and the Rust
direct-message test cover the boundary. Manual release checks still need to
confirm the chip and deletion interaction in a real WebView and verify that
Claude starts without a visible console.
