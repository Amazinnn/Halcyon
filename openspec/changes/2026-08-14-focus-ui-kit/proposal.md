## Why

Control styles are duplicated across five+ files with inconsistent details (.switch.on has two variants, .seg.on three, select three class names), and there is no design contract document. Requirement #126 asks for both code and documentation so new windows look consistent automatically.

## What Changes

- Complete control-level design tokens in styles.css :root (type scale, shadows, z-index, control sizes) without changing any existing visual value.
- Add eight reusable components under components/focus/: FocusButton, FocusToggle, FocusSegmented, FocusInput, FocusSlider, FocusSelect, FocusCard, FocusWindowFrame (migration of WindowHeader with identical props/behavior).
- Replace duplicated hand-written styles in SettingsPopover.vue, DesktopView.vue, WorkflowView.vue, ChatView.vue and the four float headers.
- Deliver docs/ui-design.md (design philosophy, tokens, component contracts, window rules) and docs/ui-maintenance.md (how to add a control/window, token impact checks, gates).
- **No visible behavior change**: components reproduce the merged existing styles exactly.

## Capabilities

### New Capabilities

- `ui-kit`: reusable Focus controls consuming one design-token source, with documented contracts.

### Modified Capabilities

- None.

## Impact

Frontend components and styles only; no Rust, schema, or window behavior changes. Domain-specific controls (music progress, pet chat button, topbar capsule) stay out of the kit.