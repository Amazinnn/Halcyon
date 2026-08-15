## 1. Rust dead code

- [x] 1.1 pets.rs: delete load_draft_package, import, import_for_agent, sheet_base64, info_for (old data_dir API), remove, animation, contains, initial_provider_for_pet, resolve, row_order field + their tests
- [x] 1.2 lib.rs: delete topbar_uses_native_composition + its test; mem::forget → let _ =; unused imports
- [x] 1.3 agents/mod.rs AGENT_ID; claude.rs first_delta_sent field
- [x] 1.4 grid.rs dead helpers (rect etc.) if product-unreferenced

## 2. Frontend dead state

- [x] 2.1 stores/agent.ts bubble state cleanup + agent.test.ts updates

## 3. Lint gates and delivery

- [x] 3.1 rust-gate.ps1: warning scan + clippy correctness (rustfmt deferred: 304 pre-existing diffs)
- [x] 3.2 Full gates: cargo test, clippy, npm test/build, schema, openspec validate, git diff --check; rebuild; Eval + STATUS + push