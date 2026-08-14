## 1. Tokens and docs

- [x] 1.1 Add control-level tokens (--fs-*, --shadow-*, --z-*, --ctrl-h-*) to styles.css :root without changing existing values
- [x] 1.2 Write docs/ui-design.md (philosophy, tokens table, component contracts, window rules)
- [x] 1.3 Write docs/ui-maintenance.md (new-control/new-window flows, token impact check, gates)

## 2. Kit components

- [x] 2.1 Add components/focus/FocusButton.vue (default/accent/ghost/danger variants, disabled, hover)
- [x] 2.2 Add FocusToggle.vue (modelValue, on/off), FocusSegmented.vue (options + modelValue)
- [x] 2.3 Add FocusInput.vue (text/number), FocusSlider.vue (range), FocusSelect.vue (native select wrapper)
- [x] 2.4 Add FocusCard.vue (glass + header/note slots), FocusWindowFrame.vue (WindowHeader migration)
- [x] 2.5 Add component tests (focus/*.test.ts) for variants, disabled state, v-model behavior

## 3. Migration

- [x] 3.1 Migrate SettingsPopover.vue to kit components; remove its duplicated control styles
- [x] 3.2 Migrate DesktopView.vue (text-input/btn/focus-mode-seg) and WorkflowView.vue (btn/ghost/seg/sel)
- [x] 3.3 Migrate ChatView.vue (ghost/agent-select) and the four float headers to FocusWindowFrame
- [x] 3.4 Gate: npm test -- --run, npm run build; commit feat(ui-kit)