## Why

Inputs with fixed/elastic widths look wrong for short content ("few characters taking a full box") and long content has no graceful overflow behavior (requirement #131). The user wants content-sized controls with hard constraints: never break the outer window frame, never squeeze sibling windows, hidden/faded text when the box cannot grow, and bounded vertical growth.

## What Changes

- FocusInput gains an `autosize` prop (field-sizing: content) with min 40px / max 100% of the container — width never exceeds the frame; text beyond the input is hidden natively (caret-visible), never overflowing.
- Multi-line inputs (chat composer, workflow prompt textarea) get a bounded max-height (~4 lines) with internal scroll.
- Display text that overflows (Agent names, run names) gets a right-edge fade (mask-image) instead of hard ellipsis where it improves the look; existing ellipsis stays elsewhere.
- Chat composer: Skills select and send/stop buttons align to the same 36px height as the input box.
- Apply autosize to short-content inputs (Agent name, URL name).
- ui-design.md gains the dynamic-size & overflow rules (content sizing, hard bounds, overflow handling).

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui-kit`: dynamic sizing and overflow rules join the kit contract.

## Impact

styles.css tokens/utilities, FocusInput, ChatView composer, WorkflowView textarea, SettingsPopover/DesktopView autosize usage, ui-design.md, kit tests.