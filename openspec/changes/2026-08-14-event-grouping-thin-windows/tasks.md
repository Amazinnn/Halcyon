## 1. Rust domains

- [ ] 1.1 Add Domain enum + domain() to CoreEvent; add exhaustiveness test
- [ ] 1.2 Write docs/architecture/event-streams-v1.md subscription matrix

## 2. Frontend thin mode

- [ ] 2.1 Add thin init option to stores/agent.ts (agent:status listener + one status query only)
- [ ] 2.2 App.vue: thin labels set (topbar/pet-bubble/grid-overlay) routes init({ thin })
- [ ] 2.3 Add agent-store thin test (no refreshCharacters; state updates from agent:status)
- [ ] 2.4 Gate: npm test -- --run, npm run build, cargo test --lib, openspec validate --specs --strict, git diff --check; commit feat(events)