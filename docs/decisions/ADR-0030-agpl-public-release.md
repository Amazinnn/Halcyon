# ADR-0030: AGPL-3.0-only for Public Releases

Status: Accepted
Date: 2026-08-11
Requirements: #101

## Context

Focus Desktop is useful as public source code, but its author also wants later
public releases to preserve reciprocal access to modifications, including when
modified code is offered over a network service. MIT already granted for the
`v1.12.8` tag cannot be withdrawn retroactively.

## Decision

1. Starting with `v1.12.9`, Focus Desktop's own source code is licensed as
   `AGPL-3.0-only`.
2. The root `LICENSE`, desktop package metadata, Rust crate metadata,
   event-schema package metadata, Settings UI, README, and third-party notice
   identify the same license.
3. The project remains public. AGPL-3.0-only is strong copyleft, not a
   non-commercial license.
4. Third-party dependencies and imported pet packs retain their own licenses.
5. The historical `v1.12.8` MIT release remains available under its original
   terms; this decision applies only to future releases.

## Consequences

Contributors and downstream users receive a clear license for current code.
Anyone needing a different commercial arrangement must contact the copyright
holder instead of assuming a separate grant. This decision does not add
runtime licensing checks or alter the product's local behavior.
