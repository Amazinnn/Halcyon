# cli-command-registry Specification

## Purpose
Makes the local control-plane command set declarative: one table drives dispatch, Agent whitelisting, auditing, and client help so adding a command is a single declaration.
## Requirements
### Requirement: Commands are declared in one registry
Every focus-cli command SHALL be declared in one static CommandSpec table that carries its match pattern, Agent-whitelist flag, help text, and handler.

#### Scenario: Dispatch from the registry
- **WHEN** a JSON request arrives
- **THEN** its command is dispatched by the first matching registry entry

### Requirement: Agent whitelist derives from the registry
The Agent whitelist SHALL be derived from registry entries flagged agent_allowed; `debug` and unknown commands stay denied.

#### Scenario: Whitelist matches registry
- **WHEN** an agent-thread request is checked
- **THEN** allowed commands equal exactly the registry entries flagged agent_allowed, with the same sub-command sets as before

### Requirement: Client help derives from the registry
focus-cli help text SHALL be generated from the registry help entries.

#### Scenario: Help lists every command
- **WHEN** focus-cli --help runs
- **THEN** every registry command appears with its help text

### Requirement: Window list in debug command derives from the window registry
The `debug windows` command SHALL iterate the window registry float set instead of a hard-coded label list.

#### Scenario: Debug reflects registry
- **WHEN** `debug windows` runs
- **THEN** it reports exactly the float labels from the window registry

### Requirement: Wire semantics are unchanged
Token check, agent-thread audit records, JSON framing, and response payload shapes SHALL remain identical.

#### Scenario: Audit still records
- **WHEN** an agent-thread call succeeds or is denied
- **THEN** one audit row is recorded exactly as before
