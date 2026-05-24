// src/controller/http/scope/router.rs

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
            ComputeZeroUseCase, GetScopeProfileUseCase,
            ListScopeProfilesUseCase, SaveScopeProfileInput,
            SaveScopeProfileUseCase,
        },
    },
    controller::http::{
        armory::payload::{Page, PageRequest},
        errors::ApiResult,
        middleware::auth::{AuthUser, require_auth},
        scope::payload::*,
    },
    domain::entities::ballistics::AdjustmentUnit,
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/zero", post(compute_zero))
        .route("/profiles", post(save_profile).get(list_profiles))
        .route("/profiles/{id}", get(get_profile))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

async fn compute_zero(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Json(req): Json<ComputeZeroRequest>,
) -> ApiResult<Json<ComputeZeroResponse>> {
    let domain_req = parse_zero_request(req)?;
    let resp = ComputeZeroUseCase::new(&state)
        .execute(domain_req)
        .await?;
    Ok(Json(ComputeZeroResponse::from(resp)))
}

async fn save_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<SaveScopeProfileRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let unit: AdjustmentUnit = req.unit.parse()?;
    let id = SaveScopeProfileUseCase::new(&state)
        .execute(SaveScopeProfileInput {
            owner_id: auth.0.id(),
            gun_id: req.gun_id,
            name: req.name,
            unit,
            click_value: req.click_value,
            elevation_max_clicks: req.elevation_max_clicks,
            windage_max_clicks: req.windage_max_clicks,
            mount_height_mm: req.mount_height_mm,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn list_profiles(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(p): Query<PageRequest>,
) -> ApiResult<Json<Page<ScopeProfileResponse>>> {
    let page = p.page;
    let size = p.size;
    let (items, total) = ListScopeProfilesUseCase::new(&state)
        .execute(auth.0.id(), p.into())
        .await?;
    Ok(Json(Page {
        items: items.iter().map(ScopeProfileResponse::from).collect(),
        total, page, size,
    }))
}

async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ScopeProfileResponse>> {
    let p = GetScopeProfileUseCase::new(&state)
        .execute(id, auth.0.id())
        .await?;
    Ok(Json(ScopeProfileResponse::from(&p)))
}