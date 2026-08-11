# Float Drag and Visible Skill Input Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the moving-only blue/white float artifact, make Skills visible user input, and hide Claude CLI consoles.

**Architecture:** Floats retain `WS_POPUP`, no-activate, and the `WM_NCCALCSIZE` override but delegate background erasure to Windows. The frontend composes a single `$skill-name  text` user message which Rust forwards unchanged. Claude keeps its resident stream-json pipes and starts with `CREATE_NO_WINDOW` on Windows.

**Tech Stack:** Vue 3, Pinia, Vitest, Tauri 2, Rust, Windows APIs.

## Global Constraints

- Only `chat`, `stats`, `music`, `pet`, and `workflow` float lifecycles change.
- Preserve raw `SetWindowPos(SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE)` in the 24 ms poller.
- Skill discovery stays provider-scoped, but Focus must not read or inject `SKILL.md` for chat.
- Do not alter existing Claude login, model, permission, session-resume, cancellation, or stream-json semantics.
- Update ADR-0028, requirements, INC-001, STATUS, and `docs/evals/` before delivery.

---

### Task 1: Correct float erase handling and improve the probe

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/drag.rs`
- Modify: `scripts/window-style-probe.ps1`

**Interfaces:**
- Produces `pub(crate) fn enforce_float_invariants(&tauri::WebviewWindow)`.
- `float_nonclient_message_result(WM_NCCALCSIZE) == Some(0)` and `float_nonclient_message_result(WM_ERASEBKGND) == None`.
- `drag_start` and `finalize` invoke the invariant once on the main thread.

- [ ] **Step 1: Write a failing Rust test**

Replace the existing expectation with:

```rust
#[test]
fn float_nonclient_handler_preserves_client_area_without_suppressing_erase() {
    assert_eq!(float_nonclient_message_result(0x0083), Some(0));
    assert_eq!(float_nonclient_message_result(0x0014), None);
    assert_eq!(float_nonclient_message_result(0x000F), None);
}
```

- [ ] **Step 2: Verify red**

Run `cd apps/desktop/src-tauri; cargo test --lib float_nonclient_handler_preserves_client_area_without_suppressing_erase`.

Expected: fail because the current mapping returns `Some(1)` for `WM_ERASEBKGND`.

- [ ] **Step 3: Implement the isolated native fix**

Keep only `WM_NCCALCSIZE` in `float_nonclient_message_result`; define:

```rust
pub(crate) fn enforce_float_invariants(w: &tauri::WebviewWindow) {
    strip_float_frame(w);
    float_noactivate(w);
}
```

Use it in all existing float lifecycle paths and in `drag_start` after the visible guard plus `finalize` before placement. Do not call it in `poller`.

Add `SetProcessDPIAware` to the probe C# declaration, call it before window enumeration, and add `[switch]$AsJson` with:

```powershell
if ($AsJson) {
    $rows | Sort-Object Kind, Title, Hwnd | ConvertTo-Json -Depth 3
} else {
    $rows | Sort-Object Kind, Title, Hwnd | Format-Table -AutoSize
}
```

- [ ] **Step 4: Verify green**

Run the focused Rust test and `powershell -ExecutionPolicy Bypass -File scripts/window-style-probe.ps1 -AsJson` with Focus running. Expected: test passes and probe emits JSON with physical-pixel host/child geometry.

- [ ] **Step 5: Commit**

Commit `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/src/drag.rs`, and `scripts/window-style-probe.ps1` as `fix(window): preserve float rendering after drag`.

### Task 2: Compose visible, atomic Skill input

**Files:**
- Create: `apps/desktop/src/lib/chat-composer.ts`
- Create: `apps/desktop/src/lib/chat-composer.test.ts`
- Modify: `apps/desktop/src/stores/agent.ts`
- Modify: `apps/desktop/src/stores/agent.test.ts`
- Modify: `apps/desktop/src/views/chat/ChatView.vue`
- Modify: `apps/desktop/src/views/chat/ChatView.test.ts`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Produces `composeSkillMessage(skill: string | null, text: string): string`.
- Produces `shouldRemoveSelectedSkill(key, text, selectionStart, selectionEnd): boolean`.
- Changes Tauri commands to `agent_start_thread(character_id, initial_message)` and `agent_send(character_id, thread_id, text)` with no `skill_name`.

- [ ] **Step 1: Write failing frontend and Rust tests**

Create this focused frontend test:

```ts
import { describe, expect, it } from "vitest";
import { composeSkillMessage, shouldRemoveSelectedSkill } from "./chat-composer";

describe("visible Skill composer", () => {
  it("prefixes the direct user message", () => {
    expect(composeSkillMessage("focus-cli", "check status")).toBe("$focus-cli  check status");
    expect(composeSkillMessage(null, "check status")).toBe("check status");
  });

  it("removes the Skill atomically at the input boundary", () => {
    expect(shouldRemoveSelectedSkill("Backspace", "", 0, 0)).toBe(true);
    expect(shouldRemoveSelectedSkill("Delete", "text", 0, 0)).toBe(true);
    expect(shouldRemoveSelectedSkill("Delete", "text", 2, 2)).toBe(false);
  });
});
```

Update the current store test to expect the Tauri call:

```ts
{ characterId: "char-a", threadId: "thread-a", text: "$focus-cli  check status" }
```

and expect the same string in visible history. Replace the Rust `apply_selected_skill` test with a direct-message pass-through test.

- [ ] **Step 2: Verify red**

Run `cd apps/desktop; npm test -- src/lib/chat-composer.test.ts src/stores/agent.test.ts src/views/chat/ChatView.test.ts`.

Expected: module-not-found and stale `skillName` expectations.

- [ ] **Step 3: Implement the direct-message boundary**

Create:

```ts
export function composeSkillMessage(skill: string | null, text: string): string {
  const body = text.trim();
  return skill ? `$${skill}  ${body}` : body;
}

export function shouldRemoveSelectedSkill(key: string, text: string, start: number, end: number): boolean {
  if (start !== end) return false;
  return (key === "Backspace" || key === "Delete") && (text.length === 0 || start === 0);
}
```

The store composes before clearing `selectedSkill`, persists the composed value, and invokes Tauri with only `text`. The chat view uses a single styled `.composer-input` wrapper containing an uneditable larger bold `.skill-chip` and a text input. Its keydown handler clears the selected Skill when the helper returns true. Agent message authors render `agent.characterName`.

Delete `skillName`, `selected_skill_prompt`, `apply_selected_skill`, and `valid_skill_name`. Keep `list_provider_skills`; Rust forwards direct text unchanged and retains a pure pass-through test.

- [ ] **Step 4: Verify green**

Run the focused Vitest command, `cd apps/desktop/src-tauri; cargo test --lib direct_user_message`, and `cd apps/desktop; npm run build`. Expected: all pass.

- [ ] **Step 5: Commit**

Commit the frontend helper/tests, store/view/tests, and `lib.rs` as `feat(chat): send visible Skill invocations`.

### Task 3: Start Claude without a visible terminal

**Files:**
- Modify: `apps/desktop/src-tauri/src/agents/claude.rs`

**Interfaces:**
- Produces `CLAUDE_CHILD_CREATION_FLAGS` and `configure_claude_child(&mut Command)` on Windows.
- Consumes the existing `command_for`, `claude_resident_args`, and piped stdio.

- [ ] **Step 1: Write the failing Windows test**

```rust
#[cfg(windows)]
#[test]
fn claude_child_uses_no_console_creation_flag() {
    assert_eq!(
        CLAUDE_CHILD_CREATION_FLAGS,
        windows::Win32::System::Threading::CREATE_NO_WINDOW.0,
    );
}
```

- [ ] **Step 2: Verify red**

Run `cd apps/desktop/src-tauri; cargo test --lib claude_child_uses_no_console_creation_flag`.

Expected: compilation failure because the creation flag is absent.

- [ ] **Step 3: Implement without altering the stream**

```rust
#[cfg(windows)]
const CLAUDE_CHILD_CREATION_FLAGS: u32 =
    windows::Win32::System::Threading::CREATE_NO_WINDOW.0;

#[cfg(windows)]
fn configure_claude_child(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CLAUDE_CHILD_CREATION_FLAGS);
}
```

Call it after `command_for(...)` and before `.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()`. On non-Windows make no change.

- [ ] **Step 4: Verify green**

Run the new test plus `claude_command_uses_stream_json_and_resumes_only_saved_sessions` and `claude_cmd_receives_prompt_over_stdin_without_shell_expansion`. Expected: all pass.

- [ ] **Step 5: Commit**

Commit `claude.rs` as `fix(agent): hide Claude child console`.

### Task 4: Record the provider contract and release evidence

**Files:**
- Create: `docs/decisions/ADR-0028-visible-skill-invocation.md`
- Create: `docs/evals/2026-08-11-float-drag-and-skill-checkpoint.md`
- Modify: `README.md`, `docs/STATUS.md`, `docs/NEXT-SESSION-PROMPT.md`, `docs/next-phase.md`, `docs/production-incidents.md`, `docs/requirements-verbatim.md`

**Interfaces:**
- ADR-0028 supersedes only ADR-0026 decision 5.
- The Eval records exact commands and marks any unobserved visual behavior `Pending`.

- [ ] **Step 1: Write the failing documentation consistency checklist**

Required assertions: ADR-0028 says `$skill-name` is direct user input and Focus never reads selected Skill content; INC-001 is not `Verified` without moved-window visual evidence; STATUS links the new Eval; the Eval contains the Claude console and all-five-float manual checks.

- [ ] **Step 2: Verify red**

Run `rg -n "ADR-0028|2026-08-11-float-drag-and-skill|direct user message" docs README.md`.

Expected: no ADR-0028 or checkpoint before documentation changes.

- [ ] **Step 3: Implement documentation**

Create ADR-0028; update #86/#87 to implemented pending verification; append the recurrence to INC-001; update only stale Skill-injection and release-checkpoint statements in status, roadmap, README, and handoff. Create the Eval with real test counts, probe evidence, and pending/manual outcomes.

- [ ] **Step 4: Full release verification**

Run in this order: `cd apps/desktop && npm test`; `cd apps/desktop && npm run build`; `cd apps/desktop/src-tauri && cargo test --lib`; `cd packages/event-schema && npm test`; `launch-focus.cmd rebuild`; `powershell -ExecutionPolicy Bypass -File scripts/window-style-probe.ps1 -AsJson`; `git diff --check`.

Expected: every command succeeds. Never record unobserved visual behavior as Pass.

- [ ] **Step 5: Commit, push, and hand off**

Commit documentation as `docs(agent): record visible Skill invocation`, push `origin/main`, confirm a clean worktree, and provide a numbered manual checklist for the five floats, Skill chip/delete behavior, raw Skill message, and Claude console invisibility.
