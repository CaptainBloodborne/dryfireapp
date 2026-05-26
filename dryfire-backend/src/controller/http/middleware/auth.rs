//! Bearer-token authentication middleware.
//!
//! Pipeline per request:
//! 1. Pull `Authorization: Bearer <token>` (or the `session` cookie).
//! 2. Parse it into a [`Token`].
//! 3. Validate signature + expiry via [`TokenHandler`].
//! 4. The token's `ident` is a session UUID — look up the session row,
//!    refuse if revoked or expired in DB.
//! 5. Load the user, refuse if blocked.
//! 6. Inject `AuthUser` into request extensions; the handler downstream
//!    pulls it with `Extension<AuthUser>`.
//!
//! Any failure produces a `401` with a coded error body.

use std::str::FromStr;

use axum::{
    extract::State,
    http::{HeaderMap, Request, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    controller::http::errors::ApiError,
    domain::{entities::user::User, services::identity::Token},
};

/// Marker type carried inside request extensions. Handlers extract it
/// to access the authenticated user without re-parsing the token.
#[derive(Clone, Debug)]
pub struct AuthUser(pub User);

#[derive(Clone, Debug)]
pub struct AuthSession {
    pub session_id: Uuid,
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    let v = headers.get(AUTHORIZATION)?.to_str().ok()?;
    v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer "))
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // 1. Pull header (could also fall back to cookie).
    let raw = extract_bearer(req.headers())
        .ok_or_else(|| ApiError::unauthorized("AUTH_MISSING", "missing bearer token"))?;

    // 2. Parse + validate signature + expiry.
    let token = Token::from_str(raw)
        .map_err(|_| ApiError::unauthorized("AUTH_INVALID_TOKEN", "token malformed"))?;
    state
        .token_handler
        .validate_token(&token)
        .await
        .map_err(|_| ApiError::unauthorized("AUTH_INVALID_TOKEN", "token invalid or expired"))?;

    // 3. ident is a session UUID — fetch the active session.
    let session_id = Uuid::parse_str(&token.ident)
        .map_err(|_| ApiError::unauthorized("AUTH_INVALID_TOKEN", "token subject is not a session id"))?;
    let session = state
        .session_repo
        .find_active(session_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::unauthorized("AUTH_SESSION_REVOKED", "session not found or revoked"))?;

    // 4. Load the user; reject blocked/missing.
    let user = state
        .user_repo
        .find_by_id(session.user_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::unauthorized("USER_NOT_FOUND", "user no longer exists"))?;
    if user.is_blocked() {
        return Err(ApiError::unauthorized("USER_BLOCKED", "user is blocked"));
    }

    // 5. Update last-visit asynchronously (don't fail the request).
    let _ = state
        .user_repo
        .touch_last_visit(user.id(), chrono::Utc::now())
        .await;

    // 6. Stash for handlers.
    req.extensions_mut().insert(AuthUser(user));
    req.extensions_mut().insert(AuthSession { session_id });

    Ok(next.run(req).await)
}

/// Like [`require_auth`] but additionally rejects non-admin users.
/// Apply *after* `require_auth` (or use it on its own — internally it
/// calls into the same logic).
pub async fn require_admin(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // We can't directly compose two middlewares from a function, so
    // re-implement the lookup. (Alternatively: nest the routes inside
    // a router that already has require_auth applied, and have this
    // middleware only check the AuthUser extension.)
    let response_or = async {
        let raw = extract_bearer(req.headers())
            .ok_or_else(|| ApiError::unauthorized("AUTH_MISSING", "missing bearer token"))?;
        let token = Token::from_str(raw)
            .map_err(|_| ApiError::unauthorized("AUTH_INVALID_TOKEN", "token malformed"))?;
        state.token_handler.validate_token(&token).await
            .map_err(|_| ApiError::unauthorized("AUTH_INVALID_TOKEN", "token invalid or expired"))?;

        let session_id = Uuid::parse_str(&token.ident)
            .map_err(|_| ApiError::unauthorized("AUTH_INVALID_TOKEN", "token subject is not a session id"))?;
        let session = state.session_repo.find_active(session_id).await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::unauthorized("AUTH_SESSION_REVOKED", "session not found or revoked"))?;

        let user = state.user_repo.find_by_id(session.user_id).await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::unauthorized("USER_NOT_FOUND", "user no longer exists"))?;
        if user.is_blocked() {
            return Err(ApiError::unauthorized("USER_BLOCKED", "user is blocked"));
        }
        if !user.is_admin() {
            return Err(ApiError::new(
                axum::http::StatusCode::FORBIDDEN,
                "ADMIN_REQUIRED",
                "admin privilege required",
            ));
        }
        Ok((user, session_id))
    }.await;

    let (user, session_id) = response_or?;

    let mut req = req;
    req.extensions_mut().insert(AuthUser(user));
    req.extensions_mut().insert(AuthSession { session_id });
    Ok(next.run(req).await)
}
