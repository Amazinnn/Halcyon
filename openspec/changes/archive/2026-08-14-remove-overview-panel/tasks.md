## 1. Remove declarations

- [x] 1.1 Remove the overview WindowSpec entry; float-set test back to five
- [x] 1.2 Remove overview from view-registry.ts and its test; git rm OverviewPanelView.vue
- [x] 1.3 Remove overview from capabilities/default.json
- [x] 1.4 Update ui-maintenance.md §3 recipe to pure text steps
- [x] 1.5 Gate: npm test -- --run, npm run build, scripts/rust-gate.ps1, openspec validate --specs --strict, git diff --check; commit feat(panel)