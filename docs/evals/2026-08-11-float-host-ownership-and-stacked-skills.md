# Eval: 2026-08-11 Float Host and Stacked Skills Checkpoint

## Scope

Requirements #86, #88, #89, and #90; ADR-0028 and ADR-0029; INC-001.

## Automated Evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Frontend tests | Pass | `apps/desktop`: 11 files, 46 tests |
| Frontend production build | Pass | `npm run build` (`vue-tsc` and Vite) |
| Rust library tests | Pass | `cargo test --lib`: 173 passed |
| Event schema | Pass | 11 valid and 4 invalid fixtures checked |
| Focus release rebuild | Pass | `launch-focus.cmd rebuild` exit code 0 after stopping the prior release process |
| Transparent-host regression | Pass | `float_nonclient_handler_preserves_transparent_float_background` |
| Stacked Skill composer | Pass | 17 focused assertions are included in the 46 frontend tests |
| Diff hygiene | Pass | `git diff --check` passed after the probe and documentation updates. |

## Float Evidence

The pre-fix capture `C:\Users\yanwei\AppData\Local\Temp\focus-pet-before-erase-fix.png`
shows the white strip inside Focus's own pet HWND. The probe for the release
host reports `WS_POPUP`, `WS_EX_NOACTIVATE`, `outer == client == 256x240`, and
`Foreground = False` for the pet and its WebView children. This established
that the visible strip was default `WM_ERASEBKGND` output in a transparent
client area, not a missing process or a lost style bit.

After the handler returned `1` for `WM_ERASEBKGND`, the release was rebuilt and
the pet was moved by a DPI-aware native `SetWindowPos(...SWP_NOACTIVATE |
SWP_ASYNCWINDOWPOS)` `+1px` and back. The Focus-only capture
`C:\Users\yanwei\AppData\Local\Temp\focus-pet-after-dpi-aware-native-move.png`
retains the sprite and transparent background with no white strip.

The repaired dynamic probe was rerun against the rebuilt release PID `39808`
for two seconds at 100 ms intervals and saved to
`C:\Users\yanwei\AppData\Local\Temp\focus-window-style-2026-08-11-release-final.json`.
`ConvertFrom-Json` parsed 825 samples (180 host rows and 645 child rows). The
five product float hosts (`对话`, `统计`, `音乐`, `桌宠`, `工作流`) remained
`WS_POPUP`, had no caption or thick-frame bits, used `WS_EX_NOACTIVATE`, and
had zero foreground samples. This is structural evidence only; it does not
replace real drag/resize visual acceptance.

## Skill Evidence

The composer stores ordered `selectedSkills`. Multiple tokens render before
the input and send as the visible user text `$skill-a  $skill-b  text`.
At the input boundary, Backspace/Delete removes exactly the adjacent token as
one unit; normal text editing remains native. Selection clears after sending,
switching agents, and starting a new thread.

## Manual Gates

Pending user confirmation: drag, resize, collapse/restore, and topmost for
`chat`, `stats`, `music`, `pet`, and `workflow`; hidden-button clickability;
real Provider receipt of stacked Skill text. Do not mark INC-001 or the visual
Skill gate `Verified` from this snapshot alone.
