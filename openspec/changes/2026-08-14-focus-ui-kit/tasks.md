## 1. Tokens and docs

- [ ] 1.1 Add control-level tokens (--fs-*, --shadow-*, --z-*, --ctrl-h-*) to styles.css :root without changing existing values
- [ ] 1.2 Write docs/ui-design.md (philosophy, tokens table, component contracts, window rules)
- [ ] 1.3 Write docs/ui-maintenance.md (new-control/new-window flows, token impact check, gates)

## 2. Kit components

- [ ] 2.1 Add components/focus/FocusButton.vue (default/accent/ghost/danger variants, disabled, hover)
- [ ] 2.2 Add FocusToggle.vue (modelValue, on/off), FocusSegmented.vue (options + modelValue)
- [ ] 2.3 Add FocusInput.vue (text/number), FocusSlider.vue (range), FocusSelect.vue (native select wrapper)
- [ ] 2.4 Add FocusCard.vue (glass + header/note slots), FocusWindowFrame.vue (WindowHeader migration)
- [ ] 2.5 Add component tests (focus/*.test.ts) for variants, disabled state, v-model behavior

## 3. Migration

- [ ] 3.1 Migrate SettingsPopover.vue to kit components; remove its duplicated control styles
- [ ] 3.2 Migrate DesktopView.vue (text-input/btn/focus-mode-seg) and WorkflowView.vue (btn/ghost/seg/sel)
- [ ] 3.3 Migrate ChatView.vue (ghost/agent-select) and the four float headers to FocusWindowFrame
- [ ] 3.4 Gate: npm test -- --run, npm run build; commit feat(ui-kit)