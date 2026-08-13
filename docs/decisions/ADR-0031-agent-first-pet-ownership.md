# ADR-0031: Agent-First Pet Ownership

Status: Accepted
Date: 2026-08-13
Requirements: #103, #104

## Context

The earlier model allowed a global pet-pack list and treated a Character as a
derivative of that package. That made it unclear which Agent owned a pet,
allowed an independent active-pet concept, and made an Agent without a pet
impossible to represent cleanly.

## Decision

1. An Agent is the primary identity. It has a persistent name, one fixed
   Provider, workspace, sessions, and optional pet package.
2. A pet package is an Agent-local, non-transferable appearance stored at
   `Focus-Agents/<agent-id>/pet-pack/`. Import replaces only that Agent's
   existing package.
3. The persisted current Agent controls chat, the desktop pet, and speech
   bubbles. A current Agent without a package renders no desktop pet or bubble.
4. Deleting an Agent removes its database row, provider sessions, local
   `pet-pack`, and every workflow owned by or targeting that Agent. Its
   workspace directory, `AGENTS.md`, and user-created files remain.
5. Settings is the only place to create, select, delete, change Provider, or
   import/remove the current Agent's pet package. There is no global pet
   activation list.
6. Existing legacy app-data pet packs are migrated once at startup into Agent
   workspaces. Runtime reads use only Agent workspace packages afterward.

## Consequences

The product can represent an Agent before it has an appearance, while the
desktop identity remains unambiguous. Removing an Agent deliberately removes
workflows that could no longer run, rather than retaining invalid schedules.
This change does not transfer packages between Agents or add a general asset
library.
