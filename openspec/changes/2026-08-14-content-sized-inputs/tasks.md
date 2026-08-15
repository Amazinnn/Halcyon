## 1. Tokens and utilities

- [ ] 1.1 Add --ctrl-min-input-auto, --ctrl-max-input-h tokens and .fade-x/.fade-y utilities to styles.css

## 2. Components

- [ ] 2.1 FocusInput autosize prop (field-sizing: content, min 40px, max-width 100%)
- [ ] 2.2 ChatView composer: max-height + internal scroll; Skills select and buttons aligned to 36px
- [ ] 2.3 WorkflowView prompt textarea: field-sizing content + bounded max-height + scroll
- [ ] 2.4 Fade overflow on SettingsPopover pack-name and run-name
- [ ] 2.5 Apply autosize to Agent name and URL name inputs

## 3. Docs and tests

- [ ] 3.1 ui-design.md dynamic-size & overflow rules
- [ ] 3.2 focus-kit tests for autosize prop and tokens; update source assertions if touched
- [ ] 3.3 Gate: npm test -- --run, npm run build, scripts/rust-gate.ps1, openspec validate --specs --strict, git diff --check; commit feat(ui-kit)