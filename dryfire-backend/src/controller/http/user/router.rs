//! User HTTP routes. Two subtrees:
//!
//! - **Public** (no auth): register, login, verify email, password reset.
//! - **Protected** (requires auth middleware): get/edit current user, logout.

use axum::{
    Extension, Json, Router,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode, header::USER_AGENT},
    middleware,
    routing::{get, post},
};
use std::net::SocketAddr;

use crate::{
    application::{
        app_state::AppState,
        user_use_cases::{
            ConfirmPasswordResetUseCase, EditUserDataUseCase, EditUserInput,
            GetCurrentUserUseCase, LoginInput, LoginUserUseCase, LogoutUseCase,
            RegisterInput, RegisterNewUserUseCase, RequestPasswordResetUseCase,
            VerifyEmailUseCase,
        },
    },
    controller::http::{
        errors::{ApiError, ApiResult},
        middleware::auth::{AuthSession, AuthUser, require_auth},
        user::payload::*,
    },
};

/// Public user routes (no auth required).
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/verify-email", get(verify_email))
        .route("/request-password-reset", post(request_password_reset))
        .route("/reset-password", post(confirm_password_reset))
}

/// Protected routes — wrapped with the auth middleware.
pub fn protected_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me).patch(edit_me))
        .route("/logout", post(logout))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

// ----------------------------- handlers ---------------------------- //

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<RegisterResponse>)> {
    let out = RegisterNewUserUseCase::new(&state)
        .execute(RegisterInput {
            login: req.login,
            firstname: req.firstname,
            surname: req.surname,
            email: req.email,
            password: req.password,
            date_of_birth: req.date_of_birth,
            region: req.region,
            language: req.language,
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: out.user_id,
            message: "verification email sent",
        }),
    ))
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(socket): ConnectInfo<SocketAddr>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let ua = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    // let ip = socket.map(|ConnectInfo(addr)| addr.ip());
    let ip = socket.ip();

    let out = LoginUserUseCase::new(&state)
        .execute(LoginInput {
            login_or_email: req.identifier,
            password: req.password,
            user_agent: ua,
            ip: Some(ip),
        })
        .await?;

    Ok(Json(LoginResponse {
        user_id: out.user_id,
        session_id: out.session_id,
        access_token: out.access_token.to_string(),
    }))
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn verify_email(
    State(state): State<AppState>,
    Query(q): Query<VerifyEmailQuery>,
) -> ApiResult<Json<VerifyEmailResponse>> {
    let user_id = VerifyEmailUseCase::new(&state).execute(&q.token).await?;
    Ok(Json(VerifyEmailResponse { user_id, message: "email verified" }))
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<RequestPasswordResetRequest>,
) -> ApiResult<StatusCode> {
    RequestPasswordResetUseCase::new(&state)
        .execute(&req.email)
        .await?;

    Ok(StatusCode::ACCEPTED)
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(req): Json<ConfirmPasswordResetRequest>,
) -> ApiResult<StatusCode> {
    ConfirmPasswordResetUseCase::new(&state)
        .execute(&req.token, req.new_password)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn get_me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Json<UserResponse>> {
    let user = GetCurrentUserUseCase::new(&state).execute(auth.0.id()).await?;
    Ok(Json(UserResponse::from(&user)))
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn edit_me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<EditUserRequest>,
) -> ApiResult<Json<UserResponse>> {
    let updated = EditUserDataUseCase::new(&state)
        .execute(
            auth.0.id(),
            EditUserInput {
                firstname: req.firstname,
                surname: req.surname,
                region: req.region,
                language: req.language,
            },
        )
        .await?;
    Ok(Json(UserResponse::from(&updated)))
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn logout(
    State(state): State<AppState>,
    Extension(session): Extension<AuthSession>,
) -> ApiResult<StatusCode> {
    LogoutUseCase::new(&state).execute(session.session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
