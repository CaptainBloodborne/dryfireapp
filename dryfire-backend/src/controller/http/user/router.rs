use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};

use crate::{
    application::app_state::AppState,
    controller::http::user::payload::{NewUserPayload},
};

pub fn user_router() -> Router<AppState> {
    let router = Router::new().route("/user", post(add_new_user));

    router
}

#[cfg_attr(feature = "debug_router", axum_macros::debug_handler)]
async fn add_new_user(
    State(app_state): State<AppState>,
    Json(payload): Json<NewUserPayload>,
) -> anyhow::Result<()> {
    Ok(())
}
