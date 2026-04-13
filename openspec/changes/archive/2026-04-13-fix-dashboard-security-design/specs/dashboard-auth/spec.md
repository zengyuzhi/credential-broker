## MODIFIED Requirements

### Requirement: PIN generation and validation
The system SHALL generate PINs only through vaultd's `POST /api/auth/challenge` endpoint, not by direct database writes. Each challenge SHALL have a unique ID. The PIN SHALL be stored as a blake3 hash tied to the challenge ID with a 5-minute expiry. The PIN SHALL be invalidated after first successful use or after 5 failed attempts against the same challenge ID.

#### Scenario: Successful PIN login with challenge ID
- **WHEN** user submits the correct PIN alongside a valid challenge ID within 5 minutes
- **THEN** a session cookie is set with `httpOnly`, `SameSite=Strict`, and a 4-hour TTL
- **THEN** the challenge is invalidated and cannot be reused
- **THEN** the user is redirected to the dashboard home page

#### Scenario: Expired challenge
- **WHEN** user submits a PIN after the challenge's 5-minute window
- **THEN** the system rejects with "Challenge expired. Run vault ui again."

#### Scenario: Brute-force protection per challenge
- **WHEN** 5 incorrect PIN attempts are made for the same challenge ID
- **THEN** the challenge is burned and returns "Too many attempts. Run vault ui again."

#### Scenario: Missing or invalid challenge ID
- **WHEN** a login request is submitted without a challenge ID or with an unknown challenge ID
- **THEN** the system rejects with 400 Bad Request

### Requirement: Challenge rate limiting
The `POST /api/auth/challenge` endpoint SHALL be rate-limited to 3 requests per minute. Excess requests SHALL return 429 Too Many Requests.

#### Scenario: Challenge rate limit exceeded
- **WHEN** more than 3 challenge requests arrive within 60 seconds
- **THEN** the server returns 429 with "Rate limit exceeded. Try again later."

### Requirement: CSRF protection on mutating routes
Every authenticated session SHALL have a random CSRF token stored server-side. All POST/PUT/DELETE dashboard routes SHALL require a valid `X-CSRF-Token` header matching the session's token. Requests without a valid CSRF token SHALL return 403 Forbidden.

#### Scenario: Valid CSRF token
- **WHEN** a mutating request includes a valid `X-CSRF-Token` header matching the session
- **THEN** the request is processed normally

#### Scenario: Missing CSRF token
- **WHEN** a POST request to a dashboard route lacks the `X-CSRF-Token` header
- **THEN** the server returns 403 Forbidden

#### Scenario: Cross-site form submission blocked
- **WHEN** a form POST originates from a different localhost port with the session cookie but without the CSRF token
- **THEN** the server returns 403 Forbidden

### Requirement: CORS and origin protection
The system SHALL set CORS headers to allow only `http://127.0.0.1:8765` as origin. All dashboard API responses SHALL include `X-Content-Type-Options: nosniff`. All mutating routes SHALL validate the `Origin` header matches `http://127.0.0.1:8765`.

#### Scenario: Cross-origin request blocked
- **WHEN** a request arrives with an `Origin` header other than `http://127.0.0.1:8765`
- **THEN** the response does not include CORS allow headers
- **THEN** mutating routes return 403 Forbidden
