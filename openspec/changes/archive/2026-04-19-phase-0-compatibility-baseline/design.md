## Context

The repository now contains a high-level architecture document and a phased rollout plan that reframe `credential-broker` as a local API control plane. The current shipped product, however, still centers on credentials, profiles, `vault run`, proxy mode, and dashboard/serve workflows. That makes Phase 0 a coordination phase: preserve the current product, clarify the trust boundary, and prevent future work from treating the compatibility path as the final architecture.

This phase touches multiple surfaces:

- repository docs (`README.md`, `docs/ARCHITECTURE.md`, `docs/plans/`)
- CLI help text in `vault-cli`
- regression checks around the current command set

Because these changes affect user understanding, security posture, and future migration language across multiple modules, a design document is warranted even though Phase 0 is intentionally light on new runtime behavior.

## Goals / Non-Goals

**Goals:**

- Preserve the current product as a supported compatibility baseline
- Make the broker trust boundary and compatibility-path story explicit in docs and help text
- Establish Phase 0 as a non-breaking foundation for later broker-first changes
- Add enough verification that terminology changes do not regress existing flows

**Non-Goals:**

- Introduce connectors, grants, sessions, or bundles into the runtime model
- Rebuild the CLI around new abstractions
- Remove `vault run`, `profile bind`, or other current workflows
- Solve long-tail integration and setup problems in this phase

## Decisions

### Decision 1: Phase 0 is a docs/help/guardrail phase, not a domain-model refactor

Phase 0 should not try to partially implement the future architecture. Its job is to stabilize the current product, publish the target direction, and set honest terminology that later phases can build on.

**Why this over a partial refactor now?**

- A partial domain shift would create migration pressure before the new model is usable
- It would mix conceptual work with production behavior changes, making regressions harder to isolate
- The architecture and phase plan already give Phase 1 a clear starting point

### Decision 2: Introduce a dedicated `compatibility-baseline` capability

Phase 0 needs a spec-level contract of its own so the compatibility baseline is explicit rather than implied by scattered documentation. A dedicated capability lets later phases refer back to a concrete baseline contract during migration work.

**Alternative considered:** only modify existing docs or help-text capabilities.

**Why not?**

- That would spread the baseline story across unrelated capabilities
- It would make it harder to answer, "What does Phase 0 promise to preserve?"

### Decision 3: Extend `cli-help-text` instead of creating a separate help-labeling capability

Compatibility labeling in `vault --help`, `vault run --help`, and `vault profile bind --help` is fundamentally a help-text concern. The correct place for those guarantees is the existing `cli-help-text` capability.

**Alternative considered:** create a new docs-only capability for CLI messaging.

**Why not?**

- Existing CLI output requirements already live in `cli-help-text`
- Reusing that capability keeps future help-output tests in one place

### Decision 4: Preserve current commands and label them honestly

Phase 0 should not remove or functionally downgrade current commands. Instead, it should preserve them while clarifying that:

- env injection is compatibility access
- brokered access is the target preferred model
- future phases will add new primitives in parallel before any forced migration

**Alternative considered:** immediately mark commands deprecated.

**Why not?**

- There is no replacement model implemented yet
- Early deprecation messaging would create user anxiety without delivering a better workflow

### Decision 5: Verification should focus on message accuracy plus non-breaking behavior

Phase 0 should verify two things:

- user-facing copy matches the architecture direction
- the current credential/profile/run/serve/ui/upgrade flows still work as the compatibility baseline

That means targeted help-text assertions plus regression-oriented workflow checks are more important here than broad new feature tests.

## Risks / Trade-offs

- **Docs-first work can feel slow** → Mitigation: tie the phase to explicit follow-on phases and a clear broker-first architecture
- **Compatibility labels may worry current users** → Mitigation: state clearly that Phase 0 is non-breaking and that no migration is required yet
- **Terminology drift between README, architecture docs, and CLI help** → Mitigation: centralize wording in the architecture doc and add help-text regression checks
- **Phase 0 could sprawl into Phase 1 concepts** → Mitigation: keep connectors, grants, sessions, and bundle runtime changes explicitly out of scope

## Migration Plan

1. Publish the architecture and phase-plan docs as the source of truth for terminology
2. Update README and CLI help text to match the compatibility-baseline language
3. Add or refresh regression checks for existing workflows
4. Ship Phase 0 as non-breaking guidance and guardrails
5. Begin Phase 1 domain-model work only after the compatibility baseline is documented and stable

Rollback is straightforward because Phase 0 does not require schema or secret-store migration. If the wording is confusing or harmful, the project can revert or revise the messaging without touching stored user state.

## Open Questions

- Should Phase 0 update only CLI help text, or should the dashboard also visibly label compatibility paths before Phase 5?
- Should the preferred future term in user-facing copy be "brokered access", "proxy access", or "capability access"?
- Should `either` mode be described as "transitional" in Phase 0, or should that stronger wording wait until a concrete broker-native replacement exists?
