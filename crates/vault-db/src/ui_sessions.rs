use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

use crate::{codec::parse_timestamp, store::Store};

/// A UI dashboard session record stored in the database.
#[derive(Debug, Clone)]
pub struct UiSession {
    pub id: String,
    pub challenge_id: String,
    pub pin_hash: String,
    pub session_token_hash: Option<String>,
    pub csrf_token: Option<String>,
    pub attempts: i64,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Store {
    /// Insert a new UI session record (challenge phase, before PIN is verified).
    pub async fn insert_ui_session(&self, session: &UiSession) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO ui_sessions
                (id, challenge_id, pin_hash, session_token_hash, csrf_token, attempts, expires_at, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(&session.id)
        .bind(&session.challenge_id)
        .bind(&session.pin_hash)
        .bind(&session.session_token_hash)
        .bind(&session.csrf_token)
        .bind(session.attempts)
        .bind(session.expires_at.to_rfc3339())
        .bind(session.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("failed to insert ui_session")?;
        Ok(())
    }

    /// Look up an unactivated session by its challenge ID (used during PIN verification).
    pub async fn get_ui_session_by_challenge_id(
        &self,
        challenge_id: &str,
    ) -> Result<Option<UiSession>> {
        let row = sqlx::query(
            r#"
            SELECT id, challenge_id, pin_hash, session_token_hash, csrf_token,
                   attempts, expires_at, created_at
            FROM ui_sessions
            WHERE challenge_id = ?1
            "#,
        )
        .bind(challenge_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query ui_session by challenge_id")?;

        row.map(map_ui_session_row).transpose()
    }

    /// Increment the failed-attempt counter for a session identified by its challenge ID.
    pub async fn increment_attempts(&self, challenge_id: &str) -> Result<()> {
        sqlx::query("UPDATE ui_sessions SET attempts = attempts + 1 WHERE challenge_id = ?1")
            .bind(challenge_id)
            .execute(&self.pool)
            .await
            .context("failed to increment ui_session attempts")?;
        Ok(())
    }

    /// Activate a session after successful PIN verification.
    ///
    /// Sets `session_token_hash` and `csrf_token`, and extends `expires_at` to 4 hours from now.
    pub async fn activate_session(
        &self,
        challenge_id: &str,
        session_token_hash: &str,
        csrf_token: &str,
    ) -> Result<()> {
        let new_expires_at = Utc::now()
            .checked_add_signed(Duration::hours(4))
            .unwrap_or_else(Utc::now);

        sqlx::query(
            r#"
            UPDATE ui_sessions
            SET session_token_hash = ?2,
                csrf_token         = ?3,
                expires_at         = ?4
            WHERE challenge_id = ?1
            "#,
        )
        .bind(challenge_id)
        .bind(session_token_hash)
        .bind(csrf_token)
        .bind(new_expires_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("failed to activate ui_session")?;
        Ok(())
    }

    /// Look up an activated session by its hashed session token (used by dashboard middleware).
    pub async fn get_session_by_token_hash(&self, token_hash: &str) -> Result<Option<UiSession>> {
        let row = sqlx::query(
            r#"
            SELECT id, challenge_id, pin_hash, session_token_hash, csrf_token,
                   attempts, expires_at, created_at
            FROM ui_sessions
            WHERE session_token_hash = ?1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query ui_session by session_token_hash")?;

        row.map(map_ui_session_row).transpose()
    }

    /// Delete all sessions whose `expires_at` is in the past.
    pub async fn delete_expired_sessions(&self) -> Result<u64> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query("DELETE FROM ui_sessions WHERE expires_at < ?1")
            .bind(now)
            .execute(&self.pool)
            .await
            .context("failed to delete expired ui_sessions")?;
        Ok(result.rows_affected())
    }
}

fn map_ui_session_row(row: sqlx::sqlite::SqliteRow) -> Result<UiSession> {
    Ok(UiSession {
        id: row.get("id"),
        challenge_id: row.get("challenge_id"),
        pin_hash: row.get("pin_hash"),
        session_token_hash: row.get("session_token_hash"),
        csrf_token: row.get("csrf_token"),
        attempts: row.get("attempts"),
        expires_at: parse_timestamp(row.get::<&str, _>("expires_at"))?,
        created_at: parse_timestamp(row.get::<&str, _>("created_at"))?,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use tempfile::TempDir;
    use uuid::Uuid;

    use crate::{Store, ui_sessions::UiSession};

    struct TestStore {
        _dir: TempDir,
        store: Store,
    }

    async fn temp_store() -> TestStore {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = format!("sqlite:{}", dir.path().join("ui_sessions.db").display());
        let store = Store::connect(&db_url).await.expect("connect store");
        TestStore { _dir: dir, store }
    }

    fn sample_session(challenge_id: &str) -> UiSession {
        let now = Utc::now();
        UiSession {
            id: Uuid::new_v4().to_string(),
            challenge_id: challenge_id.to_string(),
            pin_hash: "blake3hashofpin".to_string(),
            session_token_hash: None,
            csrf_token: None,
            attempts: 0,
            expires_at: now + Duration::minutes(10),
            created_at: now,
        }
    }

    #[tokio::test]
    async fn get_ui_session_by_challenge_id_should_return_inserted_session() {
        let ts = temp_store().await;
        let session = sample_session("challenge-abc");

        ts.store
            .insert_ui_session(&session)
            .await
            .expect("insert ui_session");

        let found = ts
            .store
            .get_ui_session_by_challenge_id("challenge-abc")
            .await
            .expect("query by challenge_id")
            .expect("session should exist");

        assert_eq!(found.id, session.id);
        assert_eq!(found.challenge_id, "challenge-abc");
        assert_eq!(found.pin_hash, "blake3hashofpin");
        assert_eq!(found.attempts, 0);
        assert!(found.session_token_hash.is_none());
        assert!(found.csrf_token.is_none());
    }

    #[tokio::test]
    async fn get_ui_session_by_challenge_id_should_return_none_for_unknown_id() {
        let ts = temp_store().await;

        let result = ts
            .store
            .get_ui_session_by_challenge_id("nonexistent")
            .await
            .expect("query");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn increment_attempts_should_increase_attempt_count() {
        let ts = temp_store().await;
        let session = sample_session("challenge-incr");

        ts.store.insert_ui_session(&session).await.expect("insert");
        ts.store
            .increment_attempts("challenge-incr")
            .await
            .expect("increment once");
        ts.store
            .increment_attempts("challenge-incr")
            .await
            .expect("increment twice");

        let found = ts
            .store
            .get_ui_session_by_challenge_id("challenge-incr")
            .await
            .expect("query")
            .expect("exists");

        assert_eq!(found.attempts, 2);
    }

    #[tokio::test]
    async fn activate_session_should_set_token_hash_csrf_and_extend_expiry() {
        let ts = temp_store().await;
        let session = sample_session("challenge-activate");
        let original_expires = session.expires_at;

        ts.store.insert_ui_session(&session).await.expect("insert");
        ts.store
            .activate_session("challenge-activate", "tokenHash123", "csrfToken456")
            .await
            .expect("activate");

        let found = ts
            .store
            .get_ui_session_by_challenge_id("challenge-activate")
            .await
            .expect("query")
            .expect("exists");

        assert_eq!(found.session_token_hash.as_deref(), Some("tokenHash123"));
        assert_eq!(found.csrf_token.as_deref(), Some("csrfToken456"));
        // expires_at should be roughly 4 hours from now — definitely later than the 10-minute
        // window set in sample_session.
        assert!(found.expires_at > original_expires);
        let delta = found.expires_at - Utc::now();
        assert!(
            delta > Duration::hours(3),
            "expected ~4h expiry, got {delta}"
        );
    }

    #[tokio::test]
    async fn get_session_by_token_hash_should_return_activated_session() {
        let ts = temp_store().await;
        let session = sample_session("challenge-bylookup");

        ts.store.insert_ui_session(&session).await.expect("insert");
        ts.store
            .activate_session("challenge-bylookup", "lookupHash789", "csrfXyz")
            .await
            .expect("activate");

        let found = ts
            .store
            .get_session_by_token_hash("lookupHash789")
            .await
            .expect("query by token hash")
            .expect("session exists");

        assert_eq!(found.challenge_id, "challenge-bylookup");
        assert_eq!(found.csrf_token.as_deref(), Some("csrfXyz"));
    }

    #[tokio::test]
    async fn get_session_by_token_hash_should_return_none_for_unactivated_session() {
        let ts = temp_store().await;
        let session = sample_session("challenge-inactive");

        ts.store.insert_ui_session(&session).await.expect("insert");

        // session_token_hash is NULL — should not match any hash lookup
        let result = ts
            .store
            .get_session_by_token_hash("anyHash")
            .await
            .expect("query");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_expired_sessions_should_remove_only_expired_rows() {
        let ts = temp_store().await;
        let now = Utc::now();

        // Insert one already-expired session and one still-valid session.
        let expired = UiSession {
            id: Uuid::new_v4().to_string(),
            challenge_id: "challenge-expired".to_string(),
            pin_hash: "h1".to_string(),
            session_token_hash: None,
            csrf_token: None,
            attempts: 0,
            expires_at: now - Duration::seconds(1),
            created_at: now - Duration::minutes(20),
        };
        let valid = UiSession {
            id: Uuid::new_v4().to_string(),
            challenge_id: "challenge-valid".to_string(),
            pin_hash: "h2".to_string(),
            session_token_hash: None,
            csrf_token: None,
            attempts: 0,
            expires_at: now + Duration::hours(1),
            created_at: now,
        };

        ts.store
            .insert_ui_session(&expired)
            .await
            .expect("insert expired");
        ts.store
            .insert_ui_session(&valid)
            .await
            .expect("insert valid");

        let deleted = ts
            .store
            .delete_expired_sessions()
            .await
            .expect("delete expired");
        assert_eq!(deleted, 1);

        let still_expired = ts
            .store
            .get_ui_session_by_challenge_id("challenge-expired")
            .await
            .expect("query");
        assert!(still_expired.is_none(), "expired session should be gone");

        let still_valid = ts
            .store
            .get_ui_session_by_challenge_id("challenge-valid")
            .await
            .expect("query");
        assert!(still_valid.is_some(), "valid session should remain");
    }
}
