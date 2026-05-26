//! License HTTP routes.
//!
//! Three subtrees mounted at `/api/v1/licenses`:
//!
//! 1. `*/licenses/*` — user CRUD on own licenses + deadlines query.
//! 2. `/types`, `/types/{id}` — read-only browse of the type catalog
//!    (auth required so we know who's reading).
//! 3. `/admin/types/*` — admin CRUD on the type catalog.

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::{
        app_state::AppState,
        license_use_cases::{
            CreateLicenseTypeInput, CreateLicenseTypeUseCase,
            DeadlinesInRangeUseCase, DeleteLicenseTypeUseCase,
            DeleteLicenseUseCase, GetLicenseTypeUseCase, GetLicenseUseCase,
            ListLicenseTypesUseCase, ListLicensesUseCase,
            RegisterLicenseInput, RegisterLicenseUseCase,
            UpdateLicenseInput, UpdateLicenseTypeInput,
            UpdateLicenseTypeUseCase, UpdateLicenseUseCase,
        },
    },
    controller::http::{
        errors::ApiResult,
        license::payload::*,
        middleware::auth::{AuthUser, require_admin, require_auth},
    },
    domain::repositories::license::LicenseFilter,
    utils::paging::{Page, PageQuery},
};

pub fn routes(state: AppState) -> Router<AppState> {
    let user_subtree = Router::new()
        .route("/licenses", get(list_licenses).post(create_license))
        .route("/licenses/deadlines", get(get_deadlines))
        .route(
            "/licenses/{id}",
            get(get_license).patch(update_license).delete(delete_license),
        )
        .route("/types", get(list_types))
        .route("/types/{id}", get(get_type))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let admin_subtree = Router::new()
        .route("/types", post(create_type))
        .route(
            "/types/{id}",
            axum::routing::patch(update_type).delete(delete_type),
        )
        .layer(middleware::from_fn_with_state(state, require_admin));

    Router::new()
        .merge(user_subtree)
        .nest("/admin", admin_subtree)
}

// ============== licenses ============== //

async fn create_license(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateLicenseRequest>,
) -> ApiResult<(StatusCode, Json<LicenseResponse>)> {
    let l = RegisterLicenseUseCase { state: &state }
        .execute(RegisterLicenseInput {
            user_id: auth.0.id(),
            license_type_id: req.license_type_id,
            license_number: req.license_number,
            issuer: req.issuer,
            issued_at: req.issued_at,
            expires_at: req.expires_at,
            notes: req.notes,
            scan_url: req.scan_url,
            gun_ids: req.gun_ids,
        }).await?;
    let today = Utc::now().date_naive();
    Ok((StatusCode::CREATED, Json(LicenseResponse::from_license(&l, today))))
}

async fn list_licenses(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(f): Query<LicenseListQuery>,
) -> ApiResult<Json<Page<LicenseResponse>>> {
    let p = page.normalized();
    let today = Utc::now().date_naive();
    let filter = LicenseFilter {
        gun_id: f.gun_id, type_id: f.type_id, expired: f.expired, q: f.q,
    };
    let (items, total) = ListLicensesUseCase { state: &state }
        .execute(auth.0.id(), &filter, today, p.limit(), p.offset()).await?;
    let resp: Vec<LicenseResponse> = items.iter()
        .map(|l| LicenseResponse::from_license(l, today))
        .collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn get_license(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LicenseResponse>> {
    let l = GetLicenseUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(Json(LicenseResponse::from_license(&l, Utc::now().date_naive())))
}

async fn update_license(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateLicenseRequest>,
) -> ApiResult<Json<LicenseResponse>> {
    let l = UpdateLicenseUseCase { state: &state }
        .execute(auth.0.id(), id, UpdateLicenseInput {
            license_type_id: req.license_type_id,
            license_number: req.license_number,
            issuer: req.issuer,
            issued_at: req.issued_at,
            expires_at: req.expires_at,
            notes: req.notes,
            scan_url: req.scan_url,
            gun_ids: req.gun_ids,
        }).await?;
    Ok(Json(LicenseResponse::from_license(&l, Utc::now().date_naive())))
}

async fn delete_license(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteLicenseUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_deadlines(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<DeadlinesQuery>,
) -> ApiResult<Json<Vec<DeadlineResponse>>> {
    let items = DeadlinesInRangeUseCase { state: &state }
        .execute(auth.0.id(), q.from, q.to).await?;
    Ok(Json(items.into_iter().map(|d| DeadlineResponse {
        license_id: d.license_id,
        license_number: d.license_number,
        issuer: d.issuer,
        expires_at: d.expires_at,
    }).collect()))
}

// ============== license types ============== //

async fn list_types(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(q): Query<LicenseTypeListQuery>,
) -> ApiResult<Json<Page<LicenseTypeResponse>>> {
    let p = page.normalized();
    let (items, total) = ListLicenseTypesUseCase { state: &state }
        .execute(q.region.as_deref(), p.limit(), p.offset()).await?;
    let resp: Vec<LicenseTypeResponse> = items.into_iter()
        .map(|entry| LicenseTypeResponse { entry })
        .collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn get_type(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LicenseTypeResponse>> {
    let t = GetLicenseTypeUseCase { state: &state }.execute(id).await?;
    Ok(Json(LicenseTypeResponse { entry: t }))
}

// ---- admin ---- //

async fn create_type(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateLicenseTypeRequest>,
) -> ApiResult<(StatusCode, Json<LicenseTypeResponse>)> {
    let t = CreateLicenseTypeUseCase { state: &state }
        .execute(auth.0.id(), CreateLicenseTypeInput {
            code: req.code, name: req.name, region: req.region,
            validity_days: req.validity_days,
            instructions: req.instructions,
        }).await?;
    Ok((StatusCode::CREATED, Json(LicenseTypeResponse { entry: t })))
}

async fn update_type(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateLicenseTypeRequest>,
) -> ApiResult<Json<LicenseTypeResponse>> {
    let t = UpdateLicenseTypeUseCase { state: &state }
        .execute(auth.0.id(), id, UpdateLicenseTypeInput {
            code: req.code, name: req.name, region: req.region,
            validity_days: req.validity_days,
            instructions: req.instructions,
        }).await?;
    Ok(Json(LicenseTypeResponse { entry: t }))
}

async fn delete_type(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteLicenseTypeUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
