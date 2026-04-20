## ADDED Requirements

### Requirement: Phase 0 publishes the compatibility baseline and target broker direction
The repository SHALL publish Phase 0 documentation that describes the current product as a compatibility baseline and the target product as a broker-first local API control plane. `README.md`, `docs/ARCHITECTURE.md`, and the Phase 0 rollout plan SHALL distinguish brokered access from compatibility access and SHALL state that env injection is weaker than the preferred brokered trust boundary.

#### Scenario: Reader finds the architecture direction from the README
- **WHEN** a user reads `README.md`
- **THEN** the README links to `docs/ARCHITECTURE.md`
- **AND** the linked architecture doc explains that brokered capability access is the target model
- **AND** the docs explain that env injection is a compatibility path rather than the preferred security model

#### Scenario: Reader sees Phase 0 scope and non-breaking intent
- **WHEN** a user reads `docs/plans/2026-04-15-capability-broker-phase-plan.md`
- **THEN** Phase 0 is described as stabilizing the current product as a compatibility baseline
- **AND** the plan states that no forced migration is required in Phase 0

### Requirement: Phase 0 preserves current workflow surfaces while relabeling them honestly
Phase 0 SHALL keep the existing credential/profile/run/serve/ui/upgrade surfaces available without requiring connectors, grants, sessions, or bundles to be adopted first. Any user-facing language added in Phase 0 SHALL clarify which surfaces are compatibility paths versus preferred brokered paths without removing the existing commands or requiring a new data model.

#### Scenario: Existing run/profile workflow still works in Phase 0
- **WHEN** an existing user follows the current `vault credential`, `vault profile`, and `vault run` flow during Phase 0
- **THEN** the commands remain available
- **AND** no connector or grant object is required before the flow can be used

#### Scenario: Existing serve/ui/upgrade workflow still works in Phase 0
- **WHEN** an existing user follows the current `vault serve`, `vault ui`, or `vault upgrade` flows during Phase 0
- **THEN** the commands remain available
- **AND** the documentation labels the current model honestly without forcing a data-model migration first
