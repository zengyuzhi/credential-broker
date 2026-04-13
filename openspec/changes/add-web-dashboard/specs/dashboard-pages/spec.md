## ADDED Requirements

### Requirement: Home page with overview
The dashboard home page at `/` SHALL display a summary: credential count, profile count, active lease count, usage chart (24h by provider), and recent sessions list.

#### Scenario: Home page renders overview
- **WHEN** authenticated user visits `/`
- **THEN** the page shows credential count, profile count, and active lease count
- **THEN** the page shows a usage bar chart grouped by provider for the last 24 hours
- **THEN** the page shows the 5 most recent lease sessions with agent name, project, and status

### Requirement: Credentials page
The credentials page at `/credentials` SHALL list all credentials with provider, label, environment, enabled status, and last-used timestamp. Secret values SHALL be masked to show only the last 4 characters.

#### Scenario: Credential list display
- **WHEN** authenticated user visits `/credentials`
- **THEN** each credential row shows provider, label, env, enabled/disabled toggle, last used time
- **THEN** the secret field shows `****...XXXX` (last 4 chars only)

#### Scenario: Enable/disable credential from dashboard
- **WHEN** user clicks the enable/disable toggle for a credential
- **THEN** the credential enabled state is updated in the database
- **THEN** the row updates via htmx swap without full page reload

### Requirement: Profiles page
The profiles page at `/profiles` SHALL list all profiles. Clicking a profile SHALL show its bindings (provider, credential label, access mode).

#### Scenario: Profile list and detail
- **WHEN** authenticated user visits `/profiles`
- **THEN** all profiles are listed with name and binding count
- **WHEN** user clicks a profile name
- **THEN** the bindings expand inline showing provider, credential label, and mode

### Requirement: Stats page
The stats page at `/stats` SHALL show usage analytics: total requests, tokens, cost per provider, filterable by provider and time range.

#### Scenario: Stats with filters
- **WHEN** authenticated user visits `/stats`
- **THEN** usage is shown aggregated by provider with request count, tokens, and cost
- **WHEN** user selects a provider filter
- **THEN** stats update to show only that provider's data

### Requirement: Sessions page
The sessions page at `/sessions` SHALL list active and recent leases with agent name, profile, project, issued/expires timestamps, and active/expired status.

#### Scenario: Active sessions display
- **WHEN** authenticated user visits `/sessions`
- **THEN** active leases (not yet expired) are shown at the top with a visual indicator
- **THEN** recently expired leases are shown below

### Requirement: No secrets in browser responses
No dashboard page or API response SHALL include raw secret values. All credential-related responses SHALL mask secrets to last 4 characters or omit them entirely.

#### Scenario: Secret masking in all responses
- **WHEN** any dashboard page or API renders credential data
- **THEN** the `secret_ref` field is never included in the HTML or JSON response
- **THEN** no response body contains patterns matching API keys (e.g., `sk-`, `key-`)
