// src/controller/http/armory/router.rs

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
        armory_use_cases::{
            AmmoStatsInput, AmmoStatsUseCase, CreateAmmoLotInput,
            CreateAmmoLotUseCase, CreateGunInput, CreateGunUseCase,
            DeleteGunUseCase, GetGunUseCase, ListGunsUseCase,
            RecordAmmoTxnInput, RecordAmmoTxnUseCase,
        },
    },
    controller::http::{
        armory::payload::*,
        errors::ApiResult,
        middleware::auth::{AuthUser, require_auth},
    },
};

pub fn gun_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_gun).get(list_guns))
        .route("/:id", get(get_gun).delete(delete_gun))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

pub fn ammo_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/lots", post(create_lot))
        .route("/transactions", post(record_txn))
        .route("/stats", get(stats))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

async fn create_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateGunRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let id = CreateGunUseCase::new(&state).execute(CreateGunInput {
        owner_id: auth.0.id(),
        manufacturer: req.manufacturer, model: req.model, serial: req.serial,
        class: req.class, caliber: req.caliber,
        date_of_purchase: req.date_of_purchase,
        photo_url: req.photo_url, notes: req.notes,
    }).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn get_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GunResponse>> {
    let g = GetGunUseCase::new(&state).execute(id, auth.0.id()).await?;
    Ok(Json(GunResponse::from(&g)))
}

async fn list_guns(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(p): Query<PageRequest>,
) -> ApiResult<Json<Page<GunResponse>>> {
    let page = p.page; let size = p.size;
    let (items, total) = ListGunsUseCase::new(&state)
        .execute(auth.0.id(), p.into()).await?;
    Ok(Json(Page {
        items: items.iter().map(GunResponse::from).collect(),
        total, page, size,
    }))
}

async fn delete_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteGunUseCase::new(&state).execute(id, auth.0.id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_lot(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateAmmoLotRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let id = CreateAmmoLotUseCase::new(&state).execute(CreateAmmoLotInput {
        owner_id: auth.0.id(),
        manufacturer: req.manufacturer, caliber: req.caliber,
        bullet_type: req.bullet_type, shell_type: req.shell_type,
        bullet_weight_grains: req.bullet_weight_grains,
        powder_charge_grains: req.powder_charge_grains,
        initial_quantity: req.initial_quantity, notes: req.notes,
    }).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn record_txn(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<RecordTxnRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let id = RecordAmmoTxnUseCase::new(&state).execute(RecordAmmoTxnInput {
        owner_id: auth.0.id(),
        lot_id: req.lot_id, gun_id: req.gun_id, kind: req.kind,
        quantity: req.quantity, happened_at: req.happened_at, notes: req.notes,
    }).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

#[derive(serde::Deserialize)]
struct StatsQuery {
    from: Option<chrono::DateTime<chrono::Utc>>,
    to:   Option<chrono::DateTime<chrono::Utc>>,
}

async fn stats(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<StatsQuery>,
) -> ApiResult<Json<crate::application::armory_use_cases::AmmoStats>> {
    let s = AmmoStatsUseCase::new(&state).execute(AmmoStatsInput {
        owner_id: auth.0.id(), from: q.from, to: q.to,
    }).await?;
    Ok(Json(s))
}