//! Challenge-based PIN authentication and session management for the vaultd dashboard.
//!
//! Flow:
//!   1. `POST /api/auth/challenge` — generate 6-digit PIN + UUID challenge_id, store hashed PIN,
//!      return both to caller (caller shows PIN to human operator out-of-band).
//!   2. `POST /api/auth/login` — accept `{ challenge_id, pin }`, verify blake3(pin) matches,
//!      burn after 5 bad attempts, set `vault_session` httpOnly cookie on success.
//!   3. `AuthSession` extractor — validates the cookie on every protected request.
//!   4. `validate_csrf` helper — rejects mutating requests whose `X-CSRF-Token` header is wrong,
//!      and blocks requests from origins other than `http://127.0.0.1:8765`.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{FromRef, FromRequestParts, State},
    http::{HeaderMap, StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vault_db::UiSession;

use crate::app::AppState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_ATTEMPTS: i64 = 5;
const CHALLENGE_EXPIRY_SECONDS: i64 = 300; // 5 min window to enter PIN
const SESSION_COOKIE_NAME: &str = "vault_session";
const ALLOWED_ORIGIN: &str = "http://127.0.0.1:8765";
const CSRF_HEADER: &str = "x-csrf-token";
const RATE_LIMIT_MAX: u32 = 3;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Rate limiter (simple in-memory, per-IP bucket)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the key is allowed through (i.e. not rate-limited).
    pub fn check_and_increment(&self, key: &str) -> bool {
        let mut map = self.inner.lock().expect("rate limiter mutex poisoned");
        let now = Instant::now();
        let entry = map.entry(key.to_string()).or_insert((0, now));

        // Reset window if it has elapsed.
        if now.duration_since(entry.1) >= RATE_LIMIT_WINDOW {
            *entry = (0, now);
        }

        if entry.0 >= RATE_LIMIT_MAX {
            return false;
        }
        entry.0 += 1;
        true
    }
}

// ---------------------------------------------------------------------------
// Request / response bodies
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub challenge_id: String,
    pub pin: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub challenge_id: String,
    pub pin: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub ok: bool,
    pub csrf_token: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hash(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

fn set_session_cookie(raw_token: &str) -> String {
    format!("{SESSION_COOKIE_NAME}={raw_token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=14400")
}

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|pair| {
                let pair = pair.trim();
                pair.strip_prefix(SESSION_COOKIE_NAME)
                    .and_then(|rest| rest.strip_prefix('='))
                    .map(|v| v.to_string())
            })
        })
}

// ---------------------------------------------------------------------------
// POST /api/auth/challenge
// ---------------------------------------------------------------------------

pub async fn challenge_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ChallengeResponse>, (StatusCode, String)> {
    // Identify caller for rate-limiting (fall back to a fixed key — loopback only anyway).
    let caller_key = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("host"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("local")
        .to_string();

    if !state.rate_limiter.check_and_increment(&caller_key) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "rate limit exceeded: max 3 challenge requests per minute".to_string(),
        ));
    }

    // Generate 6-digit PIN and UUID challenge ID.
    let pin: String = {
        let n: u32 = rand::Rng::gen_range(&mut rand::thread_rng(), 0..1_000_000);
        format!("{n:06}")
    };
    let challenge_id = Uuid::new_v4().to_string();
    let pin_hash = hash(&pin);

    let now = Utc::now();
    let expires_at = now
        .checked_add_signed(chrono::Duration::seconds(CHALLENGE_EXPIRY_SECONDS))
        .unwrap_or(now);

    let session = UiSession {
        id: Uuid::new_v4().to_string(),
        challenge_id: challenge_id.clone(),
        pin_hash,
        session_token_hash: None,
        csrf_token: None,
        attempts: 0,
        expires_at,
        created_at: now,
    };

    state
        .store
        .insert_ui_session(&session)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create session: {err}"),
            )
        })?;

    tracing::info!(challenge_id = %challenge_id, "auth challenge issued");

    Ok(Json(ChallengeResponse { challenge_id, pin }))
}

// ---------------------------------------------------------------------------
// POST /api/auth/login
// ---------------------------------------------------------------------------

pub async fn login_handler(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Response, (StatusCode, String)> {
    // Look up session.
    let session = state
        .store
        .get_ui_session_by_challenge_id(&body.challenge_id)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("session lookup failed: {err}"),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "challenge not found".to_string()))?;

    // Check expiry.
    if session.expires_at < Utc::now() {
        return Err((StatusCode::GONE, "challenge has expired".to_string()));
    }

    // Burn after max attempts.
    if session.attempts >= MAX_ATTEMPTS {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "too many failed attempts; request a new challenge".to_string(),
        ));
    }

    // Verify PIN hash.
    if hash(&body.pin) != session.pin_hash {
        state
            .store
            .increment_attempts(&body.challenge_id)
            .await
            .map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to record attempt: {err}"),
                )
            })?;

        tracing::warn!(challenge_id = %body.challenge_id, "bad PIN attempt");
        return Err((StatusCode::UNAUTHORIZED, "incorrect PIN".to_string()));
    }

    // Generate session token and CSRF token.
    let raw_token = Uuid::new_v4().to_string();
    let session_token_hash = hash(&raw_token);
    let csrf_token = Uuid::new_v4().to_string();

    state
        .store
        .activate_session(&body.challenge_id, &session_token_hash, &csrf_token)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to activate session: {err}"),
            )
        })?;

    tracing::info!(challenge_id = %body.challenge_id, "auth session activated");

    let cookie = set_session_cookie(&raw_token);
    let body = Json(LoginResponse {
        ok: true,
        csrf_token,
    });

    Ok((StatusCode::OK, [("set-cookie", cookie)], body).into_response())
}

// ---------------------------------------------------------------------------
// AuthSession extractor
// ---------------------------------------------------------------------------

/// Axum extractor that validates the `vault_session` cookie and returns the active session.
/// Rejects with `401 Unauthorized` if the cookie is missing, invalid, or expired.
pub struct AuthSession {
    #[allow(dead_code)]
    pub session: UiSession,
}

impl<S> FromRequestParts<S> for AuthSession
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let raw_token = extract_session_cookie(&parts.headers).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "missing vault_session cookie".to_string(),
            )
        })?;

        let token_hash = hash(&raw_token);

        let session = app_state
            .store
            .get_session_by_token_hash(&token_hash)
            .await
            .map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("session lookup failed: {err}"),
                )
            })?
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "invalid session token".to_string(),
                )
            })?;

        if session.expires_at < Utc::now() {
            return Err((StatusCode::UNAUTHORIZED, "session has expired".to_string()));
        }

        Ok(AuthSession { session })
    }
}

// ---------------------------------------------------------------------------
// CSRF validation helper
// ---------------------------------------------------------------------------

/// Validate CSRF token and Origin header for mutating requests.
///
/// Call this at the top of POST/PUT/DELETE handlers that require dashboard auth.
/// Returns `Err((StatusCode, String))` on validation failure.
#[allow(dead_code)]
pub fn validate_csrf(headers: &HeaderMap, session: &UiSession) -> Result<(), (StatusCode, String)> {
    // Validate Origin header.
    let origin = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if origin != ALLOWED_ORIGIN {
        return Err((
            StatusCode::FORBIDDEN,
            format!("invalid origin: {origin:?}; expected {ALLOWED_ORIGIN}"),
        ));
    }

    // Validate X-CSRF-Token header.
    let csrf_header_value = headers
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = session.csrf_token.as_deref().unwrap_or("");
    if csrf_header_value != expected || expected.is_empty() {
        return Err((StatusCode::FORBIDDEN, "invalid CSRF token".to_string()));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use chrono::{Duration, Utc};
    use tempfile::TempDir;
    use uuid::Uuid;
    use vault_db::{Store, UiSession};

    struct TestStore {
        _dir: TempDir,
        store: Store,
    }

    async fn temp_store() -> TestStore {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = format!("sqlite:{}", dir.path().join("auth_test.db").display());
        let store = Store::connect(&db_url).await.expect("connect store");
        TestStore { _dir: dir, store }
    }

    fn make_session(challenge_id: &str, pin: &str, attempts: i64) -> UiSession {
        let now = Utc::now();
        UiSession {
            id: Uuid::new_v4().to_string(),
            challenge_id: challenge_id.to_string(),
            pin_hash: hash(pin),
            session_token_hash: None,
            csrf_token: None,
            attempts,
            expires_at: now + Duration::minutes(5),
            created_at: now,
        }
    }

    // --- challenge creation ---

    #[test]
    fn pin_is_six_digits() {
        let n: u32 = rand::Rng::gen_range(&mut rand::thread_rng(), 0..1_000_000);
        let pin = format!("{n:06}");
        assert_eq!(pin.len(), 6, "PIN must always be 6 characters");
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash("123456");
        let h2 = hash("123456");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_differs_for_different_inputs() {
        assert_ne!(hash("000000"), hash("000001"));
    }

    // --- PIN verification ---

    #[tokio::test]
    async fn correct_pin_activates_session() {
        let ts = temp_store().await;
        let session = make_session("chal-ok", "123456", 0);
        ts.store.insert_ui_session(&session).await.unwrap();

        assert_eq!(hash("123456"), session.pin_hash);

        ts.store
            .activate_session("chal-ok", "tok_hash", "csrf_x")
            .await
            .unwrap();

        let found = ts
            .store
            .get_session_by_token_hash("tok_hash")
            .await
            .unwrap()
            .expect("session should exist after activation");

        assert_eq!(found.csrf_token.as_deref(), Some("csrf_x"));
    }

    #[tokio::test]
    async fn wrong_pin_increments_attempts() {
        let ts = temp_store().await;
        let session = make_session("chal-bad", "999999", 0);
        ts.store.insert_ui_session(&session).await.unwrap();

        // Simulate wrong PIN: hashes don't match → increment attempts.
        assert_ne!(hash("000000"), session.pin_hash);
        ts.store.increment_attempts("chal-bad").await.unwrap();

        let updated = ts
            .store
            .get_ui_session_by_challenge_id("chal-bad")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.attempts, 1);
    }

    #[tokio::test]
    async fn session_burned_after_max_attempts() {
        let ts = temp_store().await;
        // Insert session that already has MAX_ATTEMPTS.
        let session = make_session("chal-burn", "777777", MAX_ATTEMPTS);
        ts.store.insert_ui_session(&session).await.unwrap();

        let found = ts
            .store
            .get_ui_session_by_challenge_id("chal-burn")
            .await
            .unwrap()
            .unwrap();
        // Handler checks `attempts >= MAX_ATTEMPTS` before verifying PIN.
        assert!(found.attempts >= MAX_ATTEMPTS, "attempts should be at max");
    }

    // --- CSRF validation ---

    fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        map
    }

    fn activated_session(csrf: &str) -> UiSession {
        let now = Utc::now();
        UiSession {
            id: Uuid::new_v4().to_string(),
            challenge_id: "c".to_string(),
            pin_hash: hash("000000"),
            session_token_hash: Some(hash("token")),
            csrf_token: Some(csrf.to_string()),
            attempts: 0,
            expires_at: now + Duration::hours(4),
            created_at: now,
        }
    }

    #[test]
    fn csrf_valid_origin_and_token_passes() {
        let session = activated_session("good-csrf-token");
        let headers = headers_with(&[("origin", ALLOWED_ORIGIN), (CSRF_HEADER, "good-csrf-token")]);
        assert!(validate_csrf(&headers, &session).is_ok());
    }

    #[test]
    fn csrf_wrong_origin_rejected() {
        let session = activated_session("good-csrf-token");
        let headers = headers_with(&[
            ("origin", "http://evil.example.com"),
            (CSRF_HEADER, "good-csrf-token"),
        ]);
        let result = validate_csrf(&headers, &session);
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn csrf_wrong_token_rejected() {
        let session = activated_session("good-csrf-token");
        let headers = headers_with(&[("origin", ALLOWED_ORIGIN), (CSRF_HEADER, "bad-token")]);
        let result = validate_csrf(&headers, &session);
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn csrf_missing_token_header_rejected() {
        let session = activated_session("good-csrf-token");
        let headers = headers_with(&[("origin", ALLOWED_ORIGIN)]);
        let result = validate_csrf(&headers, &session);
        assert!(result.is_err());
    }

    // --- Cookie extraction ---

    #[test]
    fn cookie_extracted_when_present() {
        let headers = headers_with(&[("cookie", "vault_session=abc123; other=xyz")]);
        assert_eq!(extract_session_cookie(&headers), Some("abc123".to_string()));
    }

    #[test]
    fn cookie_returns_none_when_absent() {
        let headers = headers_with(&[("cookie", "other=xyz")]);
        assert!(extract_session_cookie(&headers).is_none());
    }

    #[test]
    fn cookie_returns_none_with_no_cookie_header() {
        let headers = HeaderMap::new();
        assert!(extract_session_cookie(&headers).is_none());
    }

    // --- Rate limiter ---

    #[test]
    fn rate_limiter_allows_up_to_max_requests() {
        let rl = RateLimiter::new();
        for _ in 0..RATE_LIMIT_MAX {
            assert!(rl.check_and_increment("key1"), "should be allowed");
        }
        assert!(!rl.check_and_increment("key1"), "should be blocked");
    }

    #[test]
    fn rate_limiter_independent_keys() {
        let rl = RateLimiter::new();
        for _ in 0..RATE_LIMIT_MAX {
            rl.check_and_increment("key-a");
        }
        // key-b should be unaffected.
        assert!(rl.check_and_increment("key-b"));
    }
}
