# Runtime Contract: Broker Proxy

This document describes the wire contract for the broker's proxy endpoint —
the single HTTP surface an agent uses to call an upstream provider without
ever seeing the raw API key.

## Endpoint

```
POST http://127.0.0.1:8765/v1/proxy/{provider}/{*path}
```

- `{provider}` — provider identifier registered in `vault-providers`
  (currently `openai`, `anthropic`, `twitterapi`).
- `{*path}` — the upstream path to forward to, without leading slash.
  For example `v1/chat/completions` for OpenAI, `v1/messages` for Anthropic.

The broker resolves the provider's configured `upstream_base_url` and
forwards the request body verbatim. Only providers whose adapter returns
`supports_proxy() == true` are routable.

## Authentication

The proxy accepts exactly one of two auth headers on each request:

### 1. `Authorization: Bearer <session-token>` (preferred)

Used by supported agent workflows. The session token is issued by
`vault session issue --bundle <name> --agent <name>` and bound to a
bundle that pins one or more grants. The broker:

1. Looks up the session by `blake3(token)`.
2. Rejects if the session is expired (`expires_at < now`) or revoked.
3. Enumerates every enabled grant on the session whose connector's
   `provider` matches the path `{provider}` segment, joined through
   `session_grants → grants → capabilities → connectors`, requiring
   `enabled=1` at every cascade level.
4. Re-checks `check_grant_active` defensively to catch grants whose
   `expires_at` passed while the SQL filter was running.
5. If more than one candidate remains, requires the caller to narrow
   with `X-Vault-Connector` and/or `X-Vault-Capability` — see below.
6. Enforces `grant.max_requests` as a per-session quota using the
   `session_grants.request_count` counter. Requests past the cap
   return `429 Too Many Requests`.
7. Loads the credential referenced by the picked grant's connector
   and resolves the secret from the platform keychain.

Returns `403 Forbidden` if no enabled grant authorizes that provider
for the session (or no grant matches the supplied hints); `409 Conflict`
if multiple grants match and no disambiguation header was supplied;
`429 Too Many Requests` if the per-grant quota has been exhausted;
`401 Unauthorized` if the token is unknown or expired.

#### Session issuance constraints (for completeness)

Grants are attached to a session only when they pass all three filters
at `vault session issue` time:

- `grant.agent_name == <issued-to-agent>` or `grant.agent_name == "*"`
  (cross-agent privilege leaks between `codex` and `claude` sessions
  sharing a bundle are impossible).
- `grant.enabled == 1` and `grant.expires_at > now`.
- `grant.require_confirmation == 0` (interactive approval is not yet
  wired — confirmation-required grants are skipped with a warning).

The session's own `expires_at` is capped at the tightest
`grant.ttl_minutes` across attached grants, so a 1-week session cannot
outlive a 60-minute grant cap.

### Optional disambiguation headers (session branch only)

| Header | Effect |
|--------|--------|
| `X-Vault-Connector: <name>` | Restrict candidate grants to connectors whose name matches. |
| `X-Vault-Capability: <name>` | Restrict candidate grants to capabilities whose name matches. |

When a session has multiple same-provider grants (for example two
OpenAI connectors for different accounts, or a narrow
`openai.responses.create` capability alongside a broad one), at
least one of these headers is required. The broker returns
`409 Conflict` with a list of candidates if the selection is ambiguous.

### 2. `x-vault-lease-token: <lease-token>` (compatibility)

Used by the legacy `vault run` flow. Lease tokens are issued when the
CLI spawns a child process in Proxy mode for a profile binding. The
broker looks up the lease by token hash and finds a matching binding
in `profile_bindings` where `mode ∈ {Proxy, Either}`. This path
predates sessions and does not go through the grant/capability model.

If both headers are present, the `Authorization` header wins.
If neither is present, the request fails with `401 Unauthorized`.

## Request Body

The broker forwards the raw request body to the upstream. Content-Type
from the incoming request is propagated. The broker sets the upstream
auth header itself based on the provider:

- `openai`, `twitterapi`: `Authorization: Bearer <secret>`
- `anthropic`: `x-api-key: <secret>` plus `anthropic-version: 2023-06-01`

## Response

The broker returns the upstream HTTP status and body verbatim. The
response body is not inspected for correctness — adapters only parse
it to derive usage metrics (tokens, model, cost) for telemetry.

## Errors

| Status | Meaning |
|--------|---------|
| `401 Unauthorized` | Missing/invalid/expired auth token |
| `403 Forbidden` | Token is valid but no grant authorizes this provider (or no grant matches the supplied `X-Vault-Connector`/`X-Vault-Capability` hint) |
| `409 Conflict` | Multiple grants authorize this provider; caller must disambiguate |
| `429 Too Many Requests` | Per-grant `max_requests` quota has been exhausted for this session |
| `400 Bad Request` | Unknown provider identifier |
| `502 Bad Gateway` | Provider has no upstream or upstream request failed |
| `500 Internal Server Error` | DB lookup failure or keychain error |

## Lifecycle

Every proxy call records one `UsageEvent` row in SQLite with:

- `credential_id`, `provider`, `operation`, `endpoint`, `model`
- `prompt_tokens`, `completion_tokens`, `total_tokens`, `estimated_cost_micros`
- `status_code`, `latency_ms`, `success`, `error_text`
- `agent_name`, `project` (from session or lease)
- `session_id`, `bundle_id` (session branch only)
- `lease_id` (lease branch only)

After each successful dispatch, the broker also:
- `UPDATE credentials SET last_used_at = now` for freshness tracking
  (fire-and-forget).
- `UPDATE sessions SET request_count = request_count + 1` — session-wide
  observability counter (session branch only, fire-and-forget).
- `UPDATE session_grants SET request_count = request_count + 1 WHERE ...`
  — per-grant quota counter. Only bumped on upstream `2xx` responses so
  a failed call doesn't permanently erode the caller's `max_requests`
  quota (session branch only, fire-and-forget).

All three updates are fire-and-forget; failure to write telemetry or
bump counters never fails the upstream response to the caller.

## Compatibility

The session branch (Authorization Bearer) is the supported agent
contract going forward. The lease branch (x-vault-lease-token) is
retained for existing `vault run` integrations and will remain as a
compatibility layer for humans operating legacy child-process tools.

## Example

Issue a session:

```bash
vault session issue --bundle coding-default --agent claude-code --ttl 60
# prints: token=<64-char-hex>
```

Call upstream through the broker:

```bash
curl -H 'Authorization: Bearer <token>' \
     -H 'Content-Type: application/json' \
     -d @body.json \
     http://127.0.0.1:8765/v1/proxy/openai/v1/chat/completions
```
