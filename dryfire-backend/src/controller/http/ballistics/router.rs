// src/controller/http/ballistics/router.rs

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use uuid::Uuid;

use crate::{
    application::{
        app_state::AppState,
        ballistics_use_cases::{
            ComputeTrajectoryInput, ComputeTrajectoryUseCase,
            GetBallisticProfileUseCase, ListBallisticProfilesUseCase,
            SaveBallisticProfileInput, SaveBallisticProfileUseCase,
        },
    },
    controller::http::{
        armory::payload::{Page, PageRequest},
        ballistics::payload::*,
        errors::ApiResult,
        middleware::auth::{AuthUser, require_auth},
    },
    domain::entities::ballistics::AdjustmentUnit,
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/compute", post(compute))
        .route("/profiles", post(save_profile).get(list_profiles))
        .route("/profiles/{id}", get(get_profile))
        .route("/profiles/{id}/export", get(export_profile))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

async fn compute(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Json(req): Json<ComputeTrajectoryRequest>,
) -> ApiResult<Json<ComputeTrajectoryResponse>> {
    let unit: AdjustmentUnit = req.unit.parse()?;
    let points = ComputeTrajectoryUseCase::new(&state)
        .execute(ComputeTrajectoryInput {
            input: req.input,
            env: req.env,
            unit,
            step_m: req.step_m,
            max_range_m: req.max_range_m,
        })
        .await?;
    Ok(Json(ComputeTrajectoryResponse {
        unit: unit.as_str().into(),
        points,
    }))
}

async fn save_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<SaveProfileRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let id = SaveBallisticProfileUseCase::new(&state)
        .execute(SaveBallisticProfileInput {
            owner_id: auth.0.id(),
            name: req.name,
            gun_id: req.gun_id,
            lot_id: req.lot_id,
            input: req.input,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn list_profiles(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(p): Query<PageRequest>,
) -> ApiResult<Json<Page<ProfileResponse>>> {
    let page = p.page;
    let size = p.size;
    let (items, total) = ListBallisticProfilesUseCase::new(&state)
        .execute(auth.0.id(), p.into())
        .await?;
    Ok(Json(Page {
        items: items.iter().map(ProfileResponse::from).collect(),
        total, page, size,
    }))
}

async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProfileResponse>> {
    let p = GetBallisticProfileUseCase::new(&state)
        .execute(id, auth.0.id())
        .await?;
    Ok(Json(ProfileResponse::from(&p)))
}

/// Re-runs the trajectory for a saved profile and streams the result
/// as JSON or CSV. CSV is content-type `text/csv` with
/// `Content-Disposition` set so curl etc. save it sensibly.
async fn export_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Query(q): Query<ExportQuery>,
) -> ApiResult<axum::response::Response> {
    let profile = GetBallisticProfileUseCase::new(&state)
        .execute(id, auth.0.id())
        .await?;

    let unit: AdjustmentUnit = q.unit.parse()?;
    let points = ComputeTrajectoryUseCase::new(&state)
        .execute(ComputeTrajectoryInput {
            input: profile.input.clone(),
            env: Default::default(),
            unit,
            step_m: q.step_m,
            max_range_m: q.max_range_m,
        })
        .await?;

    match q.format.as_str() {
        "csv" => {
            let body = points_to_csv(&points);
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"trajectory-{id}.csv\""),
                    ),
                ],
                body,
            ).into_response())
        }
        _ => Ok(Json(ComputeTrajectoryResponse {
            unit: unit.as_str().into(),
            points,
        })
        .into_response()),
    }
}