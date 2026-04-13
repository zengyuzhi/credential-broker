//! Static asset handlers and HTML page routes for the vaultd dashboard.
//!
//! Templates are compiled at build time via askama. CDN links (Pico CSS, htmx)
//! are embedded directly in the templates — no bundled JS/CSS assets are needed.

use askama::Template;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Login page
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub challenge_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub challenge: Option<String>,
}

/// `GET /login?challenge=<challenge_id>`
///
/// Renders the login page. The `challenge` query parameter is forwarded
/// into the form as a hidden field so the PIN submission can reference it.
pub async fn login_page(Query(params): Query<LoginQuery>) -> Response {
    let tmpl = LoginTemplate {
        challenge_id: params.challenge.unwrap_or_default(),
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("template error: {err}"),
        )
            .into_response(),
    }
}
