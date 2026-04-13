//! SSE endpoint for live dashboard updates.
//!
//! Polls SQLite every 2 seconds for changes and pushes named SSE events to
//! connected browsers. Using SQLite polling (not an in-memory broadcast channel)
//! ensures cross-process visibility: changes made by the CLI or other OS
//! processes are detected correctly.

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;

use crate::app::AppState;
use crate::auth::AuthSession;

/// Watermarks for detecting cross-process changes.
struct Watermarks {
    last_event_at: String,
    credential_count: usize,
    active_lease_count: i64,
}

/// `GET /api/events` — SSE stream (requires active session).
///
/// Sends three named event types:
/// - `stats`      — new usage event recorded
/// - `credential` — credential added / removed / toggled
/// - `lease`      — active lease count changed
pub async fn events_handler(
    _auth: AuthSession,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let store = state.store.clone();

    let stream = async_stream::stream! {
        let mut wm = Watermarks {
            last_event_at: fetch_max_event_time(&store).await,
            credential_count: fetch_credential_count(&store).await,
            active_lease_count: fetch_active_lease_count(&store).await,
        };

        let mut interval = tokio::time::interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            // Check for new usage events.
            let new_event_at = fetch_max_event_time(&store).await;
            if new_event_at != wm.last_event_at {
                wm.last_event_at = new_event_at;
                yield Ok(Event::default().event("stats").data("updated"));
            }

            // Check credential count change (add / remove / toggle).
            let new_cred_count = fetch_credential_count(&store).await;
            if new_cred_count != wm.credential_count {
                wm.credential_count = new_cred_count;
                yield Ok(Event::default().event("credential").data("updated"));
            }

            // Check active lease count change.
            let new_lease_count = fetch_active_lease_count(&store).await;
            if new_lease_count != wm.active_lease_count {
                wm.active_lease_count = new_lease_count;
                yield Ok(Event::default().event("lease").data("updated"));
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn fetch_max_event_time(store: &vault_db::Store) -> String {
    store
        .max_usage_event_time()
        .await
        .unwrap_or(None)
        .unwrap_or_default()
}

async fn fetch_credential_count(store: &vault_db::Store) -> usize {
    store.list_credentials().await.map(|v| v.len()).unwrap_or(0)
}

async fn fetch_active_lease_count(store: &vault_db::Store) -> i64 {
    store.count_active_leases().await.unwrap_or(0)
}
