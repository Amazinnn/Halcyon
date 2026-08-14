## 1. Remove declarations

- [ ] 1.1 Remove the overview WindowSpec entry; float-set test back to five
- [ ] 1.2 Remove overview from view-registry.ts and its test; git rm OverviewPanelView.vue
- [ ] 1.3 Remove overview from capabilities/default.json
- [ ] 1.4 Update ui-maintenance.md §3 recipe to pure text steps
- [ ] 1.5 Gate: npm test -- --run, npm run build, scripts/rust-gate.ps1, openspec validate --specs --strict, git diff --check; commit feat(panel)