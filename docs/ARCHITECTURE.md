# Architecture

## Status

This document describes the intended high-level architecture and design direction for `credential-broker`.
It is not a low-level implementation spec and it does not claim that the current codebase already fully matches this target architecture.
Phase 0 of this direction is intentionally non-breaking and treats today's credential/profile/run/serve model as the compatibility baseline while later phases build the broker-first model in parallel.

Related execution framing lives in [docs/plans/2026-04-15-capability-broker-phase-plan.md](./plans/2026-04-15-capability-broker-phase-plan.md).

## Design Statement

`credential-broker` should become a local API control plane for agent workflows.

At a high level, the product should combine:

- OS-backed secret custody
- brokered API access for agents
- policy and session control
- unified management and observability
- AI-native setup flows for common and custom APIs

The core idea is simple:

```text
agents should use capabilities,
not hold raw secrets
```

For the official vault-managed agent path, this is a hard invariant, not a preference.

## First-Principles Constraints

The architecture starts from a few hard constraints.

### 1. A real upstream API call eventually needs a real secret

The system cannot avoid ever handling usable credential material.
Some trusted component must eventually attach the real key, token, or credential to the outbound request.

### 2. If an agent receives the raw secret, the secret is no longer protected from that agent

This means the supported agent path MUST ensure the agent never sees the raw secret at all.
Environment-variable injection may exist as a manual compatibility escape hatch, but it is not the agent security model.

### 3. If usage should be transparent, meaningful traffic must pass through one trusted local component

Auditing only works if the product is on the actual execution path for requests or actions.

### 4. If setup should be easy, common integrations must be first-class

Users should not be required to hand-author every provider integration from scratch.

### 5. Long-tail extensibility must be data-driven

An open-source project cannot sustainably scale by shipping bespoke Rust code for every API on the internet.

## Hard Invariants

The following rules define the target trust boundary and should be treated as non-negotiable.

### 1. Agents never receive raw secrets on the official path

If a workflow is described as a supported agent integration, the agent must receive capabilities, broker endpoints, or broker-issued sessions, never raw API keys, bearer tokens, cookies, or secret-bearing files.

### 2. Agent-readable files are not secret boundaries

`.env`, JSON, YAML, shell environment, shell history, copied config snippets, prompt transcripts, and other agent-readable files are not safe homes for secrets and must not be described as protected storage.

### 3. The trusted secret boundary is intentionally narrow

For this product, the trusted homes for raw secret material are:

- the OS secret store
- the `vault` broker process
- user-triggered local input surfaces controlled by the user

### 4. `vault run` is user-only compatibility

Environment-variable injection may remain for manual user workflows, but it is not part of the supported agent security story and must never be presented as the recommended agent path.

### 5. Plaintext import is migration, not steady state

Import from `.env`, JSON, YAML, or copied config is allowed only as a user-triggered one-time migration path.
After import, the secret should move into the trusted boundary and the plaintext source should be treated as legacy material to remove.
An agent may help prepare an import plan, but the supported workflow must not require the agent to read the plaintext secret values.

## Official Security Scope

`credential-broker` guarantees only the official vault-managed path:

- the agent talks to the local broker
- the broker resolves the secret and talks to the upstream provider
- telemetry and policy decisions are recorded centrally

It does not claim to stop arbitrary same-user local processes from reading files or environment variables outside the broker.
If a user keeps secrets in `.env` files or other agent-readable plaintext storage, those secrets remain outside the supported security boundary even if the product offers a one-time migration import.

## Product Goals

### Goal 1: Safe Agent Access

Let AI agents use APIs such as OpenAI, Telegram Bot, GitHub, Twitter/X, Tavily, CoinGecko, and others without ever receiving raw secrets on the supported vault-managed path.

### Goal 2: Low-Friction Setup

Keep setup lightweight:

- install one local binary
- add a secret once
- enable a preset or import a config
- point the agent or tool at the local broker

### Goal 3: Unified Management and Observation

Provide one local control plane to manage and observe all API usage:

- see which services exist and which secrets back them
- understand which agents, profiles, and sessions can access what
- monitor request volume, failures, latency, and estimated cost
- audit sensitive actions across all configured APIs

## What The Vault Should Be

The vault should be:

- a local broker
- a local policy engine
- a local API gateway
- a local observability console
- a setup assistant for AI-agent workflows

The vault should not primarily be:

- a plaintext config manager
- a generic file vault
- a transparent network interceptor
- an env manager for agent processes
- a launcher that hands raw credentials to child processes and then steps aside

## Architectural Principles

### 1. Secrets Stay Local

Raw secrets should live in the platform secret store and in broker memory only when needed to fulfill an allowed request.
Plaintext files are not the steady-state storage model.
If a user migrates from `.env` or config files, that import should be explicit, local, and one-time.

### 2. Capabilities Beat Raw Credentials

For agent workflows this is the required security model:

- "agent may call `telegram.sendMessage`"
- not "agent receives `TELEGRAM_BOT_TOKEN`"

### 3. The Broker Is the Trust Boundary

The trusted homes for secret material are:

- the OS secret store
- the broker process
- user-triggered local input surfaces controlled by the user

Agents, prompts, environment variables, `.env` files, JSON/YAML config, logs, transcripts, and other agent-readable files are not trusted secret boundaries.

### 4. Profiles Are UX, Not the Security Boundary

Profiles remain valuable as workflow bundles and defaults.
Actual enforcement should happen through grants, policies, and sessions.

### 5. Presets First, Custom Second

Popular services should work out of the box through curated presets.
Users should be able to add new services by import or manifest without waiting for a new binary release.

### 6. Data-Driven Integrations by Default

Most integrations should be described declaratively.
Rust code should be reserved for protocol families, special auth flows, and truly custom runtime behavior.

### 7. Observation Is a Core Feature

Usage tracking, policy visibility, and auditability are central product value.
They are not optional extras layered on top of a secret store.

### 8. Explicit Integration Beats Magic Interception

The product should prefer explicit integration paths:

- known presets
- imported `curl`
- imported OpenAPI
- declarative manifests

It should avoid TLS MITM, hidden traffic capture, or brittle auto-discovery of arbitrary tool traffic.

### 9. Compatibility Paths Must Be Honest

Some user-operated tools can only work through environment-variable injection or direct credential exposure.
The product may support that path as a manual compatibility escape hatch, but it should clearly label it as weaker than brokered access and outside the supported agent security model.

## Trust and Request Flow

The core trusted flow should look like this:

```text
Agent
  -> local vault
  -> policy and session check
  -> secret resolution
  -> upstream API call
  -> telemetry and audit record
```

In this model, the secret never crosses into the agent.

## Agent Path vs User Path

The architecture should distinguish these two paths explicitly.

### Supported agent path

The supported agent path is:

```text
Agent
  -> local vault capability or gateway surface
  -> broker-issued session or policy check
  -> secret resolution inside the broker
  -> upstream API call
```

On this path, the agent never receives raw secret material.

### User-only compatibility path

`vault run` and env injection may remain available for manual user workflows that still depend on child-process credentials.
That path is not the supported agent model.
It exists only as a compatibility layer for humans operating legacy tools.

## System Overview

```text
                ┌─────────────────────────────┐
                │        User / CLI / UI      │
                │ add secret, enable preset,  │
                │ create grants, inspect use  │
                └──────────────┬──────────────┘
                               │
                               ▼
                ┌─────────────────────────────┐
                │        Control Plane        │
                │ secrets, services, grants,  │
                │ bundles, sessions, policy   │
                └──────────────┬──────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
┌────────────────┐   ┌────────────────┐   ┌────────────────┐
│   Secret Store │   │ Service Catalog│   │ Policy Engine  │
│ platform-backed│   │ presets+custom │   │ allow/deny/ttl │
└────────────────┘   └────────────────┘   └────────────────┘
                               │
                               ▼
                ┌─────────────────────────────┐
                │        Execution Plane      │
                │ proxy + actions + telemetry │
                └──────────────┬──────────────┘
                               │
                               ▼
                ┌─────────────────────────────┐
                │     Upstream Providers      │
                │ OpenAI, Telegram, GitHub,   │
                │ Twitter, Tavily, etc.       │
                └─────────────────────────────┘
```

## Core Domain Model

### Secret

Stored credential material.

Examples:

- `openai.main.api_key`
- `telegram.ops.bot_token`
- `github.personal.token`

### Connector

A reusable description of how to talk to an upstream system.

Examples:

- `openai-compatible`
- `anthropic-native`
- `telegram-bot`
- `github-rest`
- `generic-bearer-http`

### Capability

A unit of allowed behavior that an agent or session may use.

Examples:

- `openai.responses.create`
- `telegram.sendMessage`
- `github.issues.create`
- `twitter.postTweet`

### Grant

A policy object that binds capabilities to a named agent or tool identity under explicit limits such as TTL, scope, quota, or confirmation requirements.
For a low-friction baseline, grants should not require project or workspace scoping by default.

### Session

A short-lived broker-issued token that carries runtime authorization context.

### Bundle

A convenience package of grants, defaults, and workflow metadata.

This is where the current `profile` concept should evolve: a bundle is useful UX, but it should sit above grants rather than replace them.

## Two Execution Surfaces

The product should expose two different but related runtime surfaces.

### A. Model Gateway

Best for:

- LLM inference APIs
- embeddings APIs
- chat / responses APIs
- tools that already support base URL overrides

This surface is especially well-suited to OpenAI-compatible providers and similar request/response APIs.

### B. Action Gateway

Best for:

- Telegram send message
- GitHub create issue
- Twitter post tweet
- other concrete actions where capability scoping matters more than arbitrary HTTP flexibility

This surface gives the product a stronger and clearer security posture than raw proxying alone.

## Integration Strategy

The system should scale through three integration layers.

### Layer 1: Protocol Families

First-class runtime engines for common API shapes.

Examples:

- `openai-compatible`
- `anthropic-native`
- `generic-bearer-http`
- `generic-header-http`
- `query-token-http`
- `path-token-http`

These define how requests are built, authenticated, and observed.

### Layer 2: Bundled Presets

Open-source, curated definitions for popular tools and APIs.

Recommended initial set:

- OpenAI
- OpenRouter
- Anthropic
- Telegram Bot API
- GitHub REST
- Tavily
- CoinGecko
- Twitter/X-compatible providers

These presets should mostly be data files, not provider-specific application code.

### Layer 3: User-Defined Integrations

Users should be able to add long-tail services by:

- writing a manifest
- importing a working `curl`
- importing an OpenAPI description

This is the primary scaling path for custom APIs.

## Preset Catalog Strategy

Because the project is open source and owner-driven, it should include first-class support for popular APIs.
That is a feature, not a smell.

However, first-class support should mean:

- curated presets
- stable protocol families
- opinionated defaults
- community-contributed service definitions

It should not mean a bespoke Rust adapter for every service.

Example catalog shape:

```text
catalog/
  openai/openai-compatible.yaml
  anthropic/native.yaml
  telegram/bot.yaml
  github/rest.yaml
  tavily/search.yaml
  coingecko/simple-price.yaml
```

## AI-Native Setup Model

The product should support agent-assisted setup without exposing raw secrets to those agents.

The setup split should be:

- the agent configures structure
- the human provides secret material
- the vault stores the secret
- the agent uses brokered capabilities afterward

This implies a setup surface where agents can safely:

- list available presets
- inspect missing configuration
- propose bundles and grants
- import manifests or API descriptions
- run health checks

And where only the human can:

- enter raw secret values
- approve plaintext migration imports from `.env` or config files
- reveal secrets
- approve sensitive privilege expansions

If the product supports importing `.env`, YAML, or JSON files, that import must be explicitly initiated by the user through a trusted local surface.
The agent may help identify candidate variable names or draft the mapping, but the supported workflow must not require the agent to receive the plaintext file contents or secret values.

## Management and Observation Model

The system should provide one unified local view of:

- secrets and the connectors that use them
- grants and which agents or bundles depend on them
- sessions and their expiry
- request history, failures, latency, and estimated cost
- sensitive actions and policy decisions

The management layer is a first-class product requirement.

## User-Only Compatibility Mode

Some tools can only work by receiving environment variables or direct credentials.
The architecture may support this path for pragmatism, but only as a user-operated compatibility mode.

But it should be clearly framed as:

- compatibility mode
- user-only
- weaker than brokered access
- not the supported agent security story

The product should optimize for brokered access first and should never present env injection as the recommended way for an agent to access an API.

## Evolution from the Current Model

The current product model is centered on:

- credentials
- providers
- profiles
- bindings
- access modes such as `inject`, `proxy`, and `either`

The target model should evolve toward:

- secrets
- connectors
- capabilities
- grants
- sessions
- bundles

This better matches the intended product identity: a local broker and observability plane, not just a credential launcher.

## Non-Goals

This project is not intended to be:

- a cloud secret manager
- an enterprise multi-tenant IAM platform
- a fully isolated sandbox against a malicious local process running as the same OS user
- a system that keeps secrets safe while users continue storing them in agent-readable plaintext files
- a transparent interception layer for arbitrary network traffic

It is a local developer and agent control plane for safe, low-friction API access.

## Success Criteria

The architecture is succeeding when:

- common APIs work out of the box through presets
- users can add new APIs without patching Rust code
- supported agent workflows operate through brokered capabilities and never receive raw secrets
- bundles remain easy to use while policy becomes more explicit
- the user can see, in one place, what APIs exist, who can use them, and what happened

## Guiding Direction

The long-term identity of `credential-broker` should be:

```text
local secret custody
+ local API capability broker
+ local management and observability console
+ AI-native setup surface
```

That is the architectural direction this project should optimize for.
