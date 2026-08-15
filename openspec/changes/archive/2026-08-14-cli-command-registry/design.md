## Context

See proposal - Why. handle_request matches on `parts.as_slice()`; agent_whitelisted repeats sub-command sets; focus-cli print_help hard-codes usage lines. All live in the same crate, so the registry can be shared.

## Goals / Non-Goals

**Goals:** one CommandSpec table; dispatch/whitelist/help derived; debug windows uses window registry; identical wire behavior.

**Non-Goals:** no protocol v2, no version negotiation, no dynamic registration at runtime, no change to timer round-trip or workflow delegation.

## Decisions

1. **CommandSpec shape**: `{ name, matches: fn(&[&str]) -> bool, agent_allowed: bool, help: &str, handler: fn(&CommandCtx) -> Value }` where CommandCtx carries app, store, parts, payload. Function-pointer matches keep the exact existing patterns (including sub-command enums and wildcards) without inventing a pattern DSL.
2. **Order is semantic**: the table order mirrors the current match arm order; first match wins, so `workflow ..` wildcard and `debug windows` behave exactly as today.
3. **Shared with client**: bin/focus-cli.rs imports the registry (same crate) to print help; it still builds the command string the same way.
4. **debug windows**: iterates crate::window_spec float labels for the visible-window list (topbar still reported separately), removing the hard-coded four.
5. **Tests**: port the existing agent_whitelist_rules assertions to registry-derived checks; add a test that every registry handler runs for its pattern and unknown commands still fail.

## Risks / Trade-offs

- Handler signature unification may touch all 15 handlers → each handler body is moved verbatim; the CommandCtx carries exactly the inputs each currently uses.
- Order-dependence of patterns → documented in the registry header comment; the whitelist test pins sub-command sets.