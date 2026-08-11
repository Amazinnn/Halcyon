# 2026-08-11 Float Drag, Skill Input and Claude Console Checkpoint

## Scope

- Requirements: #86 and #87
- Decisions: ADR-0028; the float lifecycle remains governed by ADR-0015 and
  the existing window-management decisions.
- Commits: `288f23c` (drag erase handling), `019bb30` (visible Skill input),
  `7340eb7` (hidden Claude child console)

## Automated Evidence

| Check | Status | Evidence |
| --- | --- | --- |
| Focused frontend Skill/composer tests | Pass | `cd apps/desktop && npm test -- src/lib/chat-composer.test.ts src/stores/agent.test.ts src/views/chat/ChatView.test.ts` -> 3 files, 16 tests passed. |
| Frontend suite and production build | Pass | `cd apps/desktop && npm test` -> 11 files, 45 tests passed; `npm run build` -> `vue-tsc` and Vite build succeeded. |
| Rust direct-message boundary | Pass | `cargo test --lib direct_user_message_does_not_read_or_inject_selected_skill_content` -> 1 passed. |
| Rust Claude hidden-console flag | Pass | `cargo test --lib claude_child_uses_no_console_creation_flag` -> 1 passed on Windows. |
| Rust Claude stream regressions | Pass | `claude_command_uses_stream_json_and_resumes_only_saved_sessions` and `claude_cmd_receives_prompt_over_stdin_without_shell_expansion` -> 1 passed each. |
| Float erase regression | Pass | Focused `float_nonclient_handler_preserves_client_area_without_suppressing_erase` -> 1 passed in commit `288f23c`. |
| Rust suite | Pass | `cd apps/desktop/src-tauri && cargo test --lib` -> 171 passed, 0 failed. Existing compiler warnings remain. |
| Event schema | Pass | `cd packages/event-schema && npm test` -> 11 valid and 4 invalid fixtures checked. |
| Release rebuild | Pass | `launch-focus.cmd rebuild` exited 0 on 2026-08-11. |
| Native style probe | Pass (structural only) | `scripts/window-style-probe.ps1 -AsJson` ran against rebuilt Focus. Chat/stats/music/pet/workflow hosts were `WS_POPUP`, lacked caption/thick-frame bits, used `WS_EX_NOACTIVATE`, and none was foreground. |
| Diff hygiene | Pass | `git diff --check` passed after the documentation update. |

## Pending Manual Gates

| Gate | Status | Required observation |
| --- | --- | --- |
| Moving-window visual regression | Pending | Repeatedly open, move, resize, hide and restore chat, stats, music, pet and workflow. Confirm no blue/white border, caption, thick frame, unexpected activation, or broken hide button. |
| Native style probe | Pending | Run `powershell -ExecutionPolicy Bypass -File scripts/window-style-probe.ps1 -AsJson` while Focus is running; use it as structural evidence only, not as visual proof. |
| Visible Skill chip | Pending | Select one Skill, confirm bold larger `$skill-name` chip with spaces, press Backspace/Delete at the boundary to remove the whole atom, and send once. |
| Provider input | Pending | Confirm the provider receives exactly `$skill-name  text`, the visible history stores that same string, and no `SKILL.md` contents are appended by Focus. |
| Claude console | Pending | Start a real Claude conversation in the rebuilt release and confirm no black terminal window appears. |

## Release Commands

The full release gate for this checkpoint is:

```text
cd apps/desktop && npm test
cd apps/desktop && npm run build
cd apps/desktop/src-tauri && cargo test --lib
cd packages/event-schema && npm test
launch-focus.cmd rebuild
powershell -ExecutionPolicy Bypass -File scripts/window-style-probe.ps1 -AsJson
git diff --check
```

No pending visual or real-provider gate is marked Pass by automation alone.
