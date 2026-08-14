## Why

Adding one CLI command today requires editing three places (handle_request dispatch ~160 lines, agent_whitelisted, focus-cli help), and `debug windows` hard-codes the window list instead of consuming the window registry (ADR-0037). Requirement #127 asks for CLI extensibility so Agents can drive more Focus capabilities.

## What Changes

- Add a declarative CommandSpec registry in cli.rs: one entry declares the match pattern, agent whitelist flag, help text, and handler for each command.
- Dispatch and the Agent whitelist derive from the registry; `debug windows` iterates the window registry float set.
- focus-cli help derives from the registry (same crate), keeping the client/server command knowledge in one place.
- Wire behavior (token check, agent-thread audit, JSON protocol, response shapes) is unchanged.

## Capabilities

### New Capabilities

- `cli-command-registry`: declarative command table driving dispatch, agent whitelist, audit, and client help.

### Modified Capabilities

- None.

## Impact

cli.rs, bin/focus-cli.rs help, cli tests. No protocol change (JSON + token + audit semantics identical).