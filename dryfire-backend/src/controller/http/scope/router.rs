//! Scope routes.
//! - `POST /clicks` — click calculator
//! - `POST /rezero` — re-zero between distances.
//! - `POST /profiles`, `GET /profiles`, `GET/PATCH/DELETE /profiles/{id}` — CRUD.

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use uuid::Uuid;

use crate::{
    application::{
        app_state::AppState,
        scope_use_cases::{
            ComputeClicksUseCase, CreateScopeProfileUseCase,
            DeleteScopeProfileUseCase, GetScopeProfileUseCase,
            ListScopeProfilesUseCase, ReZeroUseCase, UpdateScopeProfileUseCase,
        },
    },
    controller::http::{
        errors::ApiResult,
        middleware::auth::{AuthUser, require_auth},
        scope::payload::*,
    },
    domain::entities::scope::ScopeProfile,
    utils::paging::{Page, PageQuery},
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/clicks", post(clicks))
        .route("/rezero", post(rezero))
        .route("/profiles", get(list_profiles).post(create_profile))
        .route(
            "/profiles/{id}",
            get(get_profile).patch(update_profile).delete(delete_profile),
        )
        .layer(middleware::from_fn_with_state(state, require_auth))
}

async fn clicks(
    Extension(_auth): Extension<AuthUser>,
    Json(req): Json<ClicksHttpRequest>,
) -> ApiResult<Json<ClicksHttpResponse>> {
    let out = ComputeClicksUseCase::execute(&req.0)?;
    Ok(Json(ClicksHttpResponse(out)))
}

async fn rezero(
    Extension(_auth): Extension<AuthUser>,
    Json(req): Json<ReZeroHttpRequest>,
) -> ApiResult<Json<ReZeroHttpResponse>> {
    let out = ReZeroUseCase::execute(&req.0)?;
    Ok(Json(ReZeroHttpResponse(out)))
}

async fn create_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateScopeProfileRequest>,
) -> ApiResult<(StatusCode, Json<ScopeProfileResponse>)> {
    let mut p = ScopeProfile::new(
        auth.0.id(), req.name, req.unit, req.click_value, req.mount_height_mm,
    );
    p.gun_id = req.gun_id;
    p.max_elevation_units = req.max_elevation_units;
    p.max_windage_units = req.max_windage_units;
    let saved = CreateScopeProfileUseCase { state: &state }.execute(p).await?;
    Ok((StatusCode::CREATED, Json(ScopeProfileResponse { profile: saved })))
}

async fn list_profiles(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Json<Page<ScopeProfile>>> {
    let p = q.normalized();
    let (items, total) = ListScopeProfilesUseCase { state: &state }
        .execute(auth.0.id(), p.limit(), p.offset())
        .await?;
    Ok(Json(Page::new(items, total, &p)))
}

async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ScopeProfileResponse>> {
    let p = GetScopeProfileUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(Json(ScopeProfileResponse { profile: p }))
}

async fn update_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateScopeProfileRequest>,
) -> ApiResult<Json<ScopeProfileResponse>> {
    let mut p = GetScopeProfileUseCase { state: &state }.execute(auth.0.id(), id).await?;
    if let Some(name) = req.name { p.name = name; }
    if let Some(g) = req.gun_id { p.gun_id = g; }
    if let Some(u) = req.unit { p.unit = u; }
    if let Some(c) = req.click_value { p.click_value = c; }
    if let Some(m) = req.max_elevation_units { p.max_elevation_units = m; }
    if let Some(m) = req.max_windage_units { p.max_windage_units = m; }
    if let Some(h) = req.mount_height_mm { p.mount_height_mm = h; }
    let saved = UpdateScopeProfileUseCase { state: &state }.execute(p).await?;
    Ok(Json(ScopeProfileResponse { profile: saved }))
}

async fn delete_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteScopeProfileUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
