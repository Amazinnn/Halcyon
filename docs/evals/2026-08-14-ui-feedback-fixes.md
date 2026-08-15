# UI Feedback Fixes Checkpoint

Date: 2026-08-14
Requirement: #132
OpenSpec change: `2026-08-14-ui-feedback-fixes`
Status: Automated gates green; Windows manual acceptance Pending

## Scope

- **Font regression (#132)**: FocusToggle/FocusSegmented/FocusSelect declared
  `font: inherit` AFTER `font-size`; the shorthand reset the size to the
  inherited default (16px). Fixed by moving `font-family: inherit` before
  `font-size` (no shorthand override). Toggles/segments return to 12px,
  selects to 11px.
- **Skills select width**: fixed at 88px (flex 0 0 88px); the opened list is
  unaffected.
- **Settings spacing + toggle-row dividers**: popover gap 12px, group
  padding-top 10px; the five toggle rows (毛玻璃/流式/提示音/顶条/桌宠背景淡化)
  render a very light bottom divider (rgba(255,255,255,0.05), 8px padding).
- ui-design.md §7 records the font contract and row-divider utility; kit test
  asserts font-size appears after font-family and no `font: inherit` remains.

## Automated Gates

| Gate | Result |
| --- | --- |
| npm test -- --run | Pass, 23 files / 130 tests |
| npm run build | Pass |
| scripts/rust-gate.ps1 | Pass (no Rust changes) |
| openspec validate | Pass |
| git diff --check | Clean |
| Release rebuild | Pending |

## Manual Acceptance (Pending)

1. Chat: Skills select ~88px (not twice the input width); list popup intact.
2. Bottom capsule 轻度/标准/学霸 at 12px; settings 毛玻璃/流式/提示音/
   自动常显隐藏/桌宠背景淡化 at 12px.
3. Settings spacing comfortable; five toggle rows show the light divider with
   breathing room.
4. No regression elsewhere; startup/focus/bubble/glass unchanged.
