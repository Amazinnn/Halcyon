## MODIFIED Requirements

### Requirement: Example overview panel exists
**Reason**: duplicate of the stats window; the user decided to remove it (requirement #129).
**Migration**: the panel recipe in docs/ui-maintenance.md §3 documents how a future panel is added by declaration and assembly; no example panel ships.

#### Scenario: No example panel ships
- **WHEN** the application starts
- **THEN** the `overview` window does not exist and the float set contains exactly chat, stats, music, pet, workflow

### Requirement: Panel recipe is documented
The recipe SHALL remain as pure text steps (window registry entry + ViewRegistry entry + capability list + read-only query + event subscription + kit assembly).

#### Scenario: Recipe without an example
- **WHEN** a developer follows ui-maintenance.md §3
- **THEN** they can add a panel window without editing existing creation, mapping, or tray logic