# Capability Broker Phase Plan

> This is a high-level product and architecture rollout plan. It is intentionally not a task-by-task implementation checklist.

## Purpose

This plan translates the architecture direction in [docs/ARCHITECTURE.md](../ARCHITECTURE.md) into phases that can be executed incrementally.

The target product is:

```text
local secret custody
+ local API capability broker
+ local management and observability console
+ AI-native setup surface
```

## Planning Principles

Each phase should:

- produce a user-visible improvement on its own
- keep the product usable between phases
- preserve a compatibility path where necessary
- reduce secret exposure over time instead of trying to achieve perfection in one jump
- add observability as part of the feature, not after it

## Phase 0: Stabilize the Current Product as the Compatibility Baseline

### Intent

Keep the existing product usable while clarifying that it is the starting point, not the final design.

### What this phase is for

- keep credential storage, profiles, `vault run`, proxy, and dashboard working
- harden release, upgrade, and storage behavior
- clearly document which current paths are compatibility paths versus target architecture
- avoid breaking existing workflows while the product direction changes

### Expected user experience

- users can continue to use the current CLI and dashboard
- users understand that env injection is supported but weaker than brokered access
- users can read one architecture doc and one phase plan and see where the product is heading

### Expected technical outcome

- current `credential` / `profile` / `run` / `serve` model remains functional
- architecture docs describe the future control-plane model honestly
- the codebase is stable enough to serve as a migration base for later phases

### What is explicitly not expected yet

- no new domain model is required yet
- no full capability system yet
- no major connector/preset redesign yet

### Exit criteria

- the current product is shippable and documented as a compatibility baseline
- the architecture and phase plan are published in-repo
- future phases can build without rewriting the story from scratch

## Phase 1: Broker Core and Domain Model Shift

### Intent

Introduce the target concepts without trying to solve every integration at once.

### What this phase is for

- define the new core concepts in product and code:
  - secret
  - connector
  - capability
  - grant
  - session
  - bundle
- preserve compatibility with current profiles while starting to reinterpret them as bundles
- establish the broker as the main trust boundary

### Expected user experience

- users can create broker-native objects, not only provider bindings
- users can understand what a connector does and what a grant allows
- users can issue short-lived broker sessions with scoped permissions

### Expected technical outcome

- the database and CLI can represent connectors, capabilities, grants, and sessions
- policies operate on allowed actions rather than only provider access mode
- the system can issue and validate runtime session tokens as a first-class primitive

### What is explicitly not expected yet

- not every provider or action needs to be migrated
- the UX does not need to be polished
- catalog/import flows can still be minimal

### Exit criteria

- the new domain model exists and is usable
- broker-issued scoped sessions exist as a stable primitive
- bundles can be mapped from or coexist with current profiles

## Phase 2: Model Gateway First

### Intent

Make brokered model access the first serious everyday path so agents can use LLM APIs without being handed raw keys.

### What this phase is for

- build the strongest and easiest broker surface first
- support the protocol families most likely to work with current agent tools
- prove that the broker model can be practical for daily AI use

### Expected scope

- OpenAI-compatible gateway
- Anthropic-native support or a clearly-scoped equivalent next family
- session-scoped auth to the local broker
- telemetry for request count, latency, failures, and cost estimates

### Expected user experience

- users can point AI tools to the local gateway instead of exporting raw keys
- a common coding-agent setup works through the broker without unusual manual steps
- usage is visible in one place

### Expected technical outcome

- the model gateway is reliable enough for normal agent workflows
- model requests are auditable end-to-end
- cost and usage data are attached to session and bundle context

### What is explicitly not expected yet

- not all non-LLM APIs need to be supported
- action-style capability granularity can remain basic here
- custom service import does not need to be complete

### Exit criteria

- at least one major agent workflow can use the local broker for model access as the default path
- brokered model access is clearly better than `vault run` for security and observability

## Phase 3: Action Gateway and Fine-Grained Capability Control

### Intent

Expand beyond model inference into APIs with real side effects, where capability control matters more than generic proxying.

### What this phase is for

- model actions as capabilities rather than just URLs
- enable safer use of APIs like Telegram, GitHub, and Twitter/X
- introduce richer grants, confirmation hooks, and scoped policies

### Expected scope

- action-oriented presets such as:
  - `telegram.sendMessage`
  - `github.issues.create`
  - `twitter.postTweet`
- capability-scoped grants
- optional human confirmation for sensitive actions
- better session and policy visibility

### Expected user experience

- users can allow an agent to do a narrow thing without exposing broad account power
- sensitive operations are visible and auditable
- the broker feels like a local control plane, not just a proxy

### Expected technical outcome

- the execution plane supports both generic proxying and named actions
- policy checks can operate at capability granularity
- audit records include action identity, actor context, and result

### What is explicitly not expected yet

- the catalog does not need to cover every popular tool
- import flows may still be rough for complex APIs
- cross-platform support is still secondary

### Exit criteria

- at least a small set of high-value action APIs can be used safely through named capabilities
- grants have enough structure to be meaningfully narrower than provider-level access

## Phase 4: Preset Catalog and AI-Native Setup

### Intent

Make setup easy enough that the product can scale beyond hand-curated built-ins without collapsing into bespoke code for every service.

### What this phase is for

- ship an opinionated preset catalog for popular APIs
- support user-defined connectors through manifests and imports
- make setup agent-assisted without exposing secrets to those agents

### Expected scope

- preset catalog for popular services
- manifest-based custom connectors
- import from `curl`
- import from OpenAPI where feasible
- setup surface that lets agents inspect missing pieces and propose configuration safely

### Expected user experience

- common services work out of the box
- new services can be added without modifying Rust code
- an agent can help set up the integration, but the human still provides the secret

### Expected technical outcome

- most integrations are data-driven
- protocol-family engines are reused by presets and imports
- the setup flow has a clean split between safe agent actions and human-only secret entry

### What is explicitly not expected yet

- not every import path needs perfect fidelity
- community ecosystem and plugin systems can wait
- enterprise-grade admin workflows are still out of scope

### Exit criteria

- the product no longer depends primarily on hardcoded provider adapters for growth
- a user can add a realistic custom service without forking the project

## Phase 5: Unified Management, Observation, and Migration of the Default UX

### Intent

Make the control plane visible and coherent enough that the new architecture becomes the obvious default product experience.

### What this phase is for

- unify management views across secrets, connectors, grants, sessions, bundles, and usage
- make policy and usage understandable from the dashboard and CLI
- shift the product default from launcher-first to broker-first

### Expected scope

- management views for:
  - secrets and attached connectors
  - grants and bundle composition
  - active sessions and expiry
  - request history and cost
  - policy decisions and denied actions
- compatibility-mode labeling for env injection
- migration path from old profiles/bindings to bundles/grants

### Expected user experience

- the user can answer:
  - what APIs do I have?
  - which agents can use them?
  - what happened yesterday?
  - what failed?
  - what cost money?
- brokered access feels like the default, understandable mode

### Expected technical outcome

- observability is a first-class part of the product model
- old and new concepts can coexist long enough for migration
- the product identity is coherent in docs, CLI, and UI

### What is explicitly not expected yet

- the legacy compatibility path does not need to disappear entirely
- advanced ecosystem work such as plugins, team mode, or remote multi-user access can wait

### Exit criteria

- the default story of the product is brokered capability access plus unified observation
- env injection is clearly secondary and optional

## Phase 6: Hardening, Ecosystem, and Platform Expansion

### Intent

After the product identity is coherent, invest in durability, distribution, and broader reach.

### What this phase is for

- cross-platform secret-store support
- packaging and install improvements
- ecosystem growth for presets and integrations
- deeper hardening around policy, confirmations, and auditing

### Expected scope

- Linux support and other platform work
- Homebrew / package-manager ergonomics
- stronger preset contribution and validation workflow
- optional advanced features such as quotas, approval flows, and richer exports

### Expected user experience

- the product is easier to adopt and easier to trust
- common workflows work across more machines and environments

### Expected technical outcome

- the architecture holds up beyond the original macOS-only early adopter audience
- the ecosystem can grow without destabilizing the broker core

### What is explicitly not expected yet

- this phase should not be used as an excuse to postpone the core broker model
- platform expansion should follow the core capability architecture, not replace it

### Exit criteria

- the product is operationally mature enough for broader real-world adoption
- ecosystem and platform work build on a stable broker-first foundation

## Summary of Phase Outcomes

### By the end of Phase 1

The project has the right language and trust boundary.

### By the end of Phase 2

The broker is useful for everyday model access.

### By the end of Phase 3

The broker is useful for real side-effect APIs with narrower permissions.

### By the end of Phase 4

The integration model scales through presets and imports.

### By the end of Phase 5

The user sees one coherent control plane for secrets, capabilities, and usage.

### By the end of Phase 6

The product is hardened, easier to adopt, and ready for broader ecosystem growth.

## Recommended Near-Term Sequencing

If the work needs to be prioritized aggressively, the recommended order is:

1. Phase 0 and Phase 1 first
2. Phase 2 immediately after, because model access is the easiest broker win
3. Phase 3 next, because action APIs are the real capability test
4. Phase 4 after that, to make growth sustainable
5. Phase 5 to unify the user-facing story
6. Phase 6 only after the broker-first identity is stable
