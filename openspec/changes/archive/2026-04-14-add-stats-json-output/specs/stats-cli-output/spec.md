## MODIFIED Requirements

### Requirement: JSON output mode for vault stats
The `vault stats` command SHALL accept a `--json` flag. When set, the command SHALL output a JSON array of provider stats objects instead of text. The JSON output SHALL respect the `--provider` filter if both flags are used.

#### Scenario: JSON output with data
- **WHEN** user runs `vault stats --json` and usage events exist
- **THEN** stdout contains a valid JSON array of objects
- **AND** each object has keys: provider, request_count, prompt_tokens, completion_tokens, total_tokens, estimated_cost_usd, last_used_at

#### Scenario: JSON output with no data
- **WHEN** user runs `vault stats --json` and no usage events exist
- **THEN** stdout contains an empty JSON array: `[]`

#### Scenario: JSON output with provider filter
- **WHEN** user runs `vault stats --json --provider openai`
- **THEN** the JSON array contains only objects where provider equals "openai"

#### Scenario: Default output unchanged
- **WHEN** user runs `vault stats` without `--json`
- **THEN** output is the existing key=value text format (no behavior change)
