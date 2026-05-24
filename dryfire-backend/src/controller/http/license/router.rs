// src/controller/http/license/router.rs

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
        license_use_cases::{
            CreateLicenseInput, CreateLicenseUseCase, DeadlinesUseCase,
            DeleteLicenseUseCase, GetLicenseUseCase, ListLicensesUseCase,
        },
    },
    controller::http::{
        armory::payload::{Page, PageRequest},
        errors::{ApiError, ApiResult},
        license::payload::*,
        middleware::auth::{AuthUser, require_auth},
    },
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create).get(list))
        .route("/deadlines", get(deadlines))
        .route("/{id}", get(get_one).delete(delete_one))
        .route("/{id}/guns", post(link_gun))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateLicenseRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let id = CreateLicenseUseCase::new(&state)
        .execute(CreateLicenseInput {
            owner_id: auth.0.id(),
            kind: req.kind,
            issuing_org: req.issuing_org,
            issued_at: req.issued_at,
            expires_at: req.expires_at,
            document_url: req.document_url,
            instructions: req.instructions,
            linked_gun_ids: req.linked_gun_ids,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(p): Query<PageRequest>,
) -> ApiResult<Json<Page<LicenseResponse>>> {
    let page = p.page;
    let size = p.size;
    let (items, total) = ListLicensesUseCase::new(&state)
        .execute(auth.0.id(), p.into())
        .await?;
    Ok(Json(Page {
        items: items.iter().map(LicenseResponse::from).collect(),
        total,
        page,
        size,
    }))
}

async fn get_one(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LicenseResponse>> {
    let lic = GetLicenseUseCase::new(&state)
        .execute(id, auth.0.id())
        .await?;
    Ok(Json(LicenseResponse::from(&lic)))
}

async fn delete_one(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteLicenseUseCase::new(&state)
        .execute(id, auth.0.id())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn deadlines(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<DeadlinesQuery>,
) -> ApiResult<Json<Vec<LicenseResponse>>> {
    if q.to < q.from {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "VALIDATION_RANGE",
            "to < from",
        ));
    }
    let items = DeadlinesUseCase::new(&state)
        .execute(auth.0.id(), q.from, q.to)
        .await?;
    Ok(Json(items.iter().map(LicenseResponse::from).collect()))
}

async fn link_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(license_id): Path<Uuid>,
    Json(req): Json<LinkGunRequest>,
) -> ApiResult<StatusCode> {
    // Resolve the license first, so we 404 on unknown / non-owned ids
    // before touching the link table.
    GetLicenseUseCase::new(&state)
        .execute(license_id, auth.0.id())
        .await?;
    state
        .license_repo
        .link_gun(license_id, req.gun_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}