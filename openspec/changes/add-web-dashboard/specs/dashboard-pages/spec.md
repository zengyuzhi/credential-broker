## ADDED Requirements

### Requirement: Home page with overview
The dashboard home page at `/` SHALL display a summary: credential count, profile count, active lease count, per-provider usage table, and recent usage events.

#### Scenario: Home page renders overview
- **WHEN** authenticated user visits `/`
- **THEN** the page shows credential count, profile count, and active lease count as overview cards
- **THEN** the page shows a per-provider usage table with request count, tokens, cost, and last used
- **THEN** the page shows the 5 most recent usage events with time, provider, agent, operation, and status

### Requirement: Credentials page
The credentials page at `/credentials` SHALL list all credentials with provider, label, environment, enabled status, and last-used timestamp. Secret values SHALL be omitted entirely from all responses.

#### Scenario: Credential list display
- **WHEN** authenticated user visits `/credentials`
- **THEN** each credential row shows provider, label, env, kind, enabled/disabled toggle, last used time
- **THEN** the secret_ref field is never included in the rendered HTML

#### Scenario: Enable/disable credential from dashboard
- **WHEN** user clicks the enable/disable toggle for a credential
- **THEN** the credential enabled state is updated in the database
- **THEN** the row updates via htmx swap without full page reload

### Requirement: Profiles page
The profiles page at `/profiles` SHALL list all profiles with expandable binding details (provider, credential ID, access mode).

#### Scenario: Profile list and detail
- **WHEN** authenticated user visits `/profiles`
- **THEN** all profiles are listed with name, description, default project, and creation date
- **WHEN** user clicks the "Bindings" summary for a profile
- **THEN** the bindings expand inline showing provider, credential ID (truncated), and mode

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
No dashboard page or API response SHALL include raw secret values. All credential-related responses SHALL omit secret fields entirely.

#### Scenario: Secret masking in all responses
- **WHEN** any dashboard page or API renders credential data
- **THEN** the `secret_ref` field is never included in the HTML or JSON response
- **THEN** no response body contains patterns matching API keys (e.g., `sk-`, `key-`)
