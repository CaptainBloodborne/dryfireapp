//! Ballistics routes.
//!
//! - `POST /compute` — ad-hoc trajectory; takes full input, returns the
//!    table. Optional `?format=csv` switches the response body to
//!    text/csv (spec requires both JSON and CSV).
//! - `POST /profiles`, `GET /profiles`, `GET /profiles/{id}`,
//!   `PATCH /profiles/{id}`, `DELETE /profiles/{id}` — CRUD on saved
//!   profiles. All scoped to the authenticated user.
//! - `POST /profiles/{id}/compute` — convenience: compute using the
//!   profile's saved bullet/sight/atmosphere, with caller-supplied
//!   wind + steps.

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    application::{
        app_state::AppState,
        ballistics_use_cases::{
            ComputeTrajectoryUseCase, CreateBallisticProfileUseCase,
            DeleteBallisticProfileUseCase, GetBallisticProfileUseCase,
            ListBallisticProfilesUseCase, UpdateBallisticProfileUseCase,
        },
    },
    controller::http::{
        ballistics::payload::*,
        errors::{ApiError, ApiResult},
        middleware::auth::{AuthUser, require_auth},
    },
    domain::{
        entities::ballistics::BallisticProfile,
        services::ballistics::{TrajectoryRequest, trajectory_to_csv},
    },
    utils::paging::{Page, PageQuery},
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/compute", post(compute_trajectory))
        .route("/profiles", get(list_profiles).post(create_profile))
        .route(
            "/profiles/{id}",
            get(get_profile).patch(update_profile).delete(delete_profile),
        )
        .route("/profiles/{id}/compute", post(compute_via_profile))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

#[derive(Debug, Deserialize)]
struct FormatQuery { format: Option<String> }

async fn compute_trajectory(
    Extension(_auth): Extension<AuthUser>,
    Query(fmt): Query<FormatQuery>,
    Json(req): Json<ComputeTrajectoryRequest>,
) -> ApiResult<Response> {
    let domain_req = TrajectoryRequest {
        bullet: req.bullet,
        sight: req.sight,
        atmosphere: req.atmosphere,
        wind: req.wind,
        steps_m: req.steps_m,
    };
    let traj = ComputeTrajectoryUseCase::execute(&domain_req)?;
    Ok(respond_traj(traj, fmt.format.as_deref()))
}

async fn compute_via_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Query(fmt): Query<FormatQuery>,
    Json(extra): Json<ComputeViaProfileRequest>,
) -> ApiResult<Response> {
    let profile = GetBallisticProfileUseCase { state: &state }
        .execute(auth.0.id(), id)
        .await?;
    let domain_req = TrajectoryRequest {
        bullet: profile.bullet,
        sight: profile.sight,
        atmosphere: extra.atmosphere.unwrap_or(profile.default_atmosphere),
        wind: extra.wind,
        steps_m: extra.steps_m,
    };
    let traj = ComputeTrajectoryUseCase::execute(&domain_req)?;
    Ok(respond_traj(traj, fmt.format.as_deref()))
}

#[derive(Debug, Deserialize)]
struct ComputeViaProfileRequest {
    #[serde(default)]
    pub atmosphere: Option<crate::domain::services::ballistics::Atmosphere>,
    pub wind: crate::domain::services::ballistics::Wind,
    pub steps_m: Vec<f64>,
}

fn respond_traj(
    traj: crate::domain::services::ballistics::Trajectory,
    format: Option<&str>,
) -> Response {
    match format {
        Some("csv") => {
            let body = trajectory_to_csv(&traj);
            (
                StatusCode::OK,
                [(CONTENT_TYPE, "text/csv; charset=utf-8")],
                body,
            ).into_response()
        }
        _ => Json(ComputeTrajectoryResponse { points: traj.points }).into_response(),
    }
}

// ---- CRUD ---- //

async fn create_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateBallisticProfileRequest>,
) -> ApiResult<(StatusCode, Json<BallisticProfileResponse>)> {
    let mut p = BallisticProfile::new(auth.0.id(), req.name, req.bullet, req.sight);
    p.gun_id = req.gun_id;
    p.ammo_id = req.ammo_id;
    p.default_atmosphere = req.default_atmosphere;
    let saved = CreateBallisticProfileUseCase { state: &state }.execute(p).await?;
    Ok((StatusCode::CREATED, Json(BallisticProfileResponse { profile: saved })))
}

async fn list_profiles(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<PageQuery>,
) -> ApiResult<Json<Page<BallisticProfile>>> {
    let p = q.normalized();
    let (items, total) = ListBallisticProfilesUseCase { state: &state }
        .execute(auth.0.id(), p.limit(), p.offset())
        .await?;
    Ok(Json(Page::new(items, total, &p)))
}

async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BallisticProfileResponse>> {
    let p = GetBallisticProfileUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(Json(BallisticProfileResponse { profile: p }))
}

async fn update_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateBallisticProfileRequest>,
) -> ApiResult<Json<BallisticProfileResponse>> {
    // Load - patch - write. Use case layer is intentionally dumb here;
    // a richer impl could merge in SQL.
    let mut p = GetBallisticProfileUseCase { state: &state }.execute(auth.0.id(), id).await?;
    if let Some(name) = req.name { p.name = name; }
    if let Some(g) = req.gun_id { p.gun_id = g; }
    if let Some(a) = req.ammo_id { p.ammo_id = a; }
    if let Some(b) = req.bullet { p.bullet = b; }
    if let Some(s) = req.sight { p.sight = s; }
    if let Some(atm) = req.default_atmosphere { p.default_atmosphere = atm; }
    let saved = UpdateBallisticProfileUseCase { state: &state }.execute(p).await?;
    Ok(Json(BallisticProfileResponse { profile: saved }))
}

async fn delete_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteBallisticProfileUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
