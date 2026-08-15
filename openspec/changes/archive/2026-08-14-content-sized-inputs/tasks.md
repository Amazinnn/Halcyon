## 1. Tokens and utilities

- [x] 1.1 Add --ctrl-min-input-auto, --ctrl-max-input-h tokens and .fade-x/.fade-y utilities to styles.css

## 2. Components

- [x] 2.1 FocusInput autosize prop (field-sizing: content, min 40px, max-width 100%)
- [x] 2.2 ChatView composer: max-height + internal scroll; Skills select and buttons aligned to 36px
- [x] 2.3 WorkflowView prompt textarea: field-sizing content + bounded max-height + scroll
- [x] 2.4 Fade overflow on SettingsPopover pack-name and run-name
- [x] 2.5 Apply autosize to Agent name and URL name inputs

## 3. Docs and tests

- [x] 3.1 ui-design.md dynamic-size & overflow rules
- [x] 3.2 focus-kit tests for autosize prop and tokens; update source assertions if touched
- [x] 3.3 Gate: npm test -- --run, npm run build, scripts/rust-gate.ps1, openspec validate --specs --strict, git diff --check; commit feat(ui-kit)