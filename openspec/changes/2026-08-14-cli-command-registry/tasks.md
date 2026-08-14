## 1. Registry

- [ ] 1.1 Define CommandSpec/CommandCtx and the registry table (ping, debug windows, timer, stats, desktop, apps, workflow, agent) in cli.rs
- [ ] 1.2 Move each handler body verbatim into its registry handler fn
- [ ] 1.3 Rewrite dispatch and agent_whitelisted to derive from the registry
- [ ] 1.4 Make debug windows iterate window_spec float labels
- [ ] 1.5 Generate focus-cli help from the registry (bin/focus-cli.rs)
- [ ] 1.6 Port whitelist tests and add dispatch/help tests; gate: cargo test --lib, openspec validate --specs --strict, git diff --check; commit feat(cli)