### Requirement: SSE event stream endpoint
The system SHALL provide a `GET /api/events` endpoint that streams Server-Sent Events to authenticated clients. The event source SHALL be SQLite polling (every 2 seconds) so that CLI-originated mutations are visible to the dashboard.

#### Scenario: CLI credential change appears in SSE
- **WHEN** a user runs `vault credential disable <id>` in the CLI
- **THEN** the dashboard receives an `event: credential` SSE message within 4 seconds

#### Scenario: CLI vault run creates lease visible in SSE
- **WHEN** a user runs `vault run --profile coding --agent codex -- <cmd>`
- **THEN** the dashboard receives an `event: lease` SSE message within 4 seconds

### Requirement: Stats update events
The system SHALL push `event: stats` messages with updated provider rollup data by polling `usage_events` for new rows since the last check, at 2-second intervals.

#### Scenario: Stats refresh after proxy request
- **WHEN** a proxy request completes and records a usage event
- **THEN** an `event: stats` SSE message is pushed within 4 seconds
