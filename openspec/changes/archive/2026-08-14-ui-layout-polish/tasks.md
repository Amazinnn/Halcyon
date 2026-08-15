## 1. Tokens and components

- [x] 1.1 Add --ctrl-min-input, --ctrl-min-select, --text-min-row tokens to styles.css :root
- [x] 1.2 FocusInput/FocusSelect apply their minimum-width tokens

## 2. SettingsPopover layout

- [x] 2.1 Agent create row: name input full-width line; provider select + add button line
- [x] 2.2 pack-row flex-wrap + row-gap; pack-name minimum width
- [x] 2.3 Audit remaining flex rows; wrap rows with real overflow risk only

## 3. Docs and tests

- [x] 3.1 ui-design.md layout & text-width section; ui-maintenance.md token note for new controls
- [x] 3.2 Update kit/migration tests for new tokens
- [x] 3.3 Gate: npm test -- --run, npm run build, scripts/rust-gate.ps1, openspec validate --specs --strict, git diff --check; commit feat(ui-kit)