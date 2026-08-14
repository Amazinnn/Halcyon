## Context

See proposal - Why. CoreEvent already uses colon namespaces in event_name(); domain() formalizes grouping. App.vue calls useAgentStore().init() for every window; TopbarView consumes agent.state from the agent:status listener. PetBubbleView is already thin (own listeners, no agent store).

## Goals / Non-Goals

**Goals:** CoreEvent domain(); event-streams doc with subscription matrix; agent store thin mode; App.vue per-label init; identical behavior.

**Non-Goals:** no event renames/repackaging, no message-bus rewrite, no changes to pet-bubble (already thin), stats/music stay on full init (out of scope).

## Decisions

1. **domain() enum**: Domain { Focus, Stats, Agent, Workflow, Supervision, Pet, Music, Probe, Panel } mapping each variant; used by docs and future filtering only.
2. **Thin mode**: stores/agent.ts init({ thin }) registers only agent:status + one agent_status invoke; full init unchanged. App.vue picks thin for topbar/pet-bubble/grid-overlay by label (pet-bubble currently runs the full init from App.vue even though its view is self-contained; thin removes that waste).
3. **Subscription matrix doc**: docs/architecture/event-streams-v1.md lists each CoreEvent + frontend listen sites (agent:event, bubble:requested, pet:state_changed, focus:tick, focus:state_changed, supervision:status, workflow:*, settings:acrylic-changed, window:visibility, stats:changed).
4. **Tests**: a Rust test asserts domain() covers every variant (match exhaustiveness gives this for free); frontend agent-store thin test asserts no refreshCharacters call and state still updates from agent:status.

## Risks / Trade-offs

- Topbar state dot depends on thin listener correctness → thin mode keeps exactly the agent:status listener; TopbarView tests already cover rendering.
- Per-label init logic in App.vue is a small map → kept as a const set of thin labels next to view-registry usage.