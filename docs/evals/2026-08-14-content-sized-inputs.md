# Content-Sized Inputs Checkpoint

Date: 2026-08-14
Requirement: #131
OpenSpec change: `2026-08-14-content-sized-inputs`
Status: Automated gates green; Windows manual acceptance Pending (field-sizing/mask behavior must be verified on the real release)

## Scope

Content-adaptive input sizing with hard overflow constraints:

- FocusInput `autosize` prop (field-sizing: content; floor 40px, cap 100% of
  container — never breaks the frame; text beyond the input hides natively).
- Multi-line inputs bounded: chat composer and workflow prompt textarea cap at
  `--ctrl-max-input-h` (~4 lines) with internal scroll.
- Display-text overflow fades: Agent names (pack-name) and run names fade at
  the right edge via mask gradients instead of hard ellipsis.
- Chat composer alignment: Skills select and send/stop buttons share the 36px
  input height.
- autosize applied to short-content inputs (Agent name, URL name).
- ui-design.md §6 documents the three overflow states (native hide / fade /
  internal scroll) and the hard bound rules.

## Automated Gates

| Gate | Result |
| --- | --- |
| npm test -- --run | Pass, 23 files / 129 tests |
| npm run build | Pass |
| scripts/rust-gate.ps1 | Pass (no Rust changes; regression check) |
| openspec validate | Pass |
| git diff --check | Clean |
| Release rebuild | Pending |

## Manual Acceptance (Pending)

1. Settings > Agent: name input shrinks for short text, grows with content,
   long names stay inside the frame (no overflow beyond the box).
2. Desktop > Add URL: name input sizes to content.
3. Settings > Agent rows: long names fade at the right edge; buttons wrap.
4. Chat: Skills select and input box same height; composer scrolls internally
   beyond ~4 lines without breaking the window frame.
5. Workflow: prompt textarea grows with lines, scrolls beyond the cap.
6. No regression in other inputs; startup/focus/bubble/glass unchanged.

## Notes

field-sizing and mask-image are progressive enhancements; older WebView2
runtimes fall back to the previous fixed widths/ellipsis. The manual
checklist verifies the real release behavior.
