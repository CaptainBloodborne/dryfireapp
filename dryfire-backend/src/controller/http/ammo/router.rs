//! Ammo HTTP routes.
//!
//! User-facing subtree (`require_auth`):
//!   POST /transactions, GET /transactions
//!   GET  /stocks
//!   GET  /stats/by-caliber, /stats/by-gun, /stats/usage
//!   GET  /types, GET /types/{id}
//!
//! Admin subtree (`require_admin`):
//!   POST /admin/types, PATCH/DELETE /admin/types/{id}

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
        ammo_use_cases::{
            CreateAmmoTypeInput, CreateAmmoTypeUseCase, DeleteAmmoTypeUseCase,
            GetAmmoTypeUseCase, ListAmmoTypesUseCase, ListStocksUseCase,
            ListTransactionsUseCase, RecordTransactionInput,
            RecordTransactionUseCase, SummaryByCaliberUseCase,
            SummaryByGunUseCase, UpdateAmmoTypeInput, UpdateAmmoTypeUseCase,
            UsageOverTimeUseCase,
        },
        app_state::AppState,
    },
    controller::http::{
        ammo::payload::*,
        errors::ApiResult,
        middleware::auth::{AuthUser, require_admin, require_auth},
    },
    domain::repositories::ammo::{
        AmmoTypeFilter, StockFilter, TransactionFilter,
    },
    utils::paging::{Page, PageQuery},
};

pub fn routes(state: AppState) -> Router<AppState> {
    let user_subtree = Router::new()
        .route("/transactions", get(list_transactions).post(record_transaction))
        .route("/stocks", get(list_stocks))
        .route("/stats/by-caliber", get(stats_by_caliber))
        .route("/stats/by-gun", get(stats_by_gun))
        .route("/stats/usage", get(stats_usage))
        .route("/types", get(list_types))
        .route("/types/{id}", get(get_type))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let admin_subtree = Router::new()
        .route("/types", post(create_type))
        .route("/types/{id}", axum::routing::patch(update_type).delete(delete_type))
        .layer(middleware::from_fn_with_state(state, require_admin));

    Router::new()
        .merge(user_subtree)
        .nest("/admin", admin_subtree)
}

// transactions

async fn record_transaction(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<RecordTransactionRequest>,
) -> ApiResult<(StatusCode, Json<TransactionResponse>)> {
    let occurred_at = req.occurred_at.unwrap_or_else(Utc::now);
    let (tx, stock) = RecordTransactionUseCase { state: &state }
        .execute(RecordTransactionInput {
            user_id: auth.0.id(),
            ammo_type_id: req.ammo_type_id,
            gun_id: req.gun_id,
            delta: req.delta,
            occurred_at,
            note: req.note,
        }).await?;
    Ok((StatusCode::CREATED, Json(TransactionResponse {
        transaction: tx,
        resulting_stock: AmmoStockResponse::from(stock),
    })))
}

async fn list_transactions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(f): Query<TransactionListQuery>,
) -> ApiResult<Json<Page<crate::domain::entities::ammo::AmmoTransaction>>> {
    let p = page.normalized();
    let filter = TransactionFilter {
        ammo_type_id: f.ammo_type_id,
        gun_id: f.gun_id,
        from: f.from,
        to: f.to,
        direction: f.direction.map(Into::into),
    };
    let (items, total) = ListTransactionsUseCase { state: &state }
        .execute(auth.0.id(), &filter, p.limit(), p.offset()).await?;
    Ok(Json(Page::new(items, total, &p)))
}

// stocks

async fn list_stocks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(f): Query<StockListQuery>,
) -> ApiResult<Json<Page<StockWithTypeResponse>>> {
    let p = page.normalized();
    let filter = StockFilter {
        ammo_type_id: f.ammo_type_id,
        caliber: f.caliber,
        include_zero: f.include_zero,
    };
    let (items, total) = ListStocksUseCase { state: &state }
        .execute(auth.0.id(), &filter, p.limit(), p.offset()).await?;
    let resp: Vec<StockWithTypeResponse> = items.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(resp, total, &p)))
}

// stats

async fn stats_by_caliber(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Json<Vec<CaliberSummaryResponse>>> {
    let items = SummaryByCaliberUseCase { state: &state }
        .execute(auth.0.id(), q.from, q.to).await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

async fn stats_by_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<RangeQuery>,
) -> ApiResult<Json<Vec<GunUsageResponse>>> {
    let items = SummaryByGunUseCase { state: &state }
        .execute(auth.0.id(), q.from, q.to).await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

async fn stats_usage(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(q): Query<UsageQuery>,
) -> ApiResult<Json<Vec<UsageBucketResponse>>> {
    let items = UsageOverTimeUseCase { state: &state }
        .execute(
            auth.0.id(),
            q.bucket.into(),
            q.from, q.to,
            q.ammo_type_id,
            q.caliber.as_deref(),
        ).await?;
    Ok(Json(items.into_iter().map(Into::into).collect()))
}

// ammo type catalog

async fn list_types(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(f): Query<AmmoTypeListQuery>,
) -> ApiResult<Json<Page<AmmoTypeResponse>>> {
    let p = page.normalized();
    let filter = AmmoTypeFilter {
        manufacturer: f.manufacturer,
        caliber: f.caliber,
        bullet_type: f.bullet_type,
        projectile_type: f.projectile_type,
        powder_charge_min: f.powder_charge_min,
        powder_charge_max: f.powder_charge_max,
        q: f.q,
    };
    let (items, total) = ListAmmoTypesUseCase { state: &state }
        .execute(&filter, p.limit(), p.offset()).await?;
    let resp: Vec<AmmoTypeResponse> = items.into_iter()
        .map(|entry| AmmoTypeResponse { entry })
        .collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn get_type(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AmmoTypeResponse>> {
    let t = GetAmmoTypeUseCase { state: &state }.execute(id).await?;
    Ok(Json(AmmoTypeResponse { entry: t }))
}

// admin

async fn create_type(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateAmmoTypeRequest>,
) -> ApiResult<(StatusCode, Json<AmmoTypeResponse>)> {
    let t = CreateAmmoTypeUseCase { state: &state }
        .execute(auth.0.id(), CreateAmmoTypeInput {
            manufacturer: req.manufacturer, name: req.name,
            caliber: req.caliber,
            bullet_type: req.bullet_type, projectile_type: req.projectile_type,
            powder_charge_grain: req.powder_charge_grain,
            bullet_weight_grain: req.bullet_weight_grain,
            notes: req.notes,
        }).await?;
    Ok((StatusCode::CREATED, Json(AmmoTypeResponse { entry: t })))
}

async fn update_type(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAmmoTypeRequest>,
) -> ApiResult<Json<AmmoTypeResponse>> {
    let t = UpdateAmmoTypeUseCase { state: &state }
        .execute(auth.0.id(), id, UpdateAmmoTypeInput {
            manufacturer: req.manufacturer, name: req.name,
            caliber: req.caliber,
            bullet_type: req.bullet_type, projectile_type: req.projectile_type,
            powder_charge_grain: req.powder_charge_grain,
            bullet_weight_grain: req.bullet_weight_grain,
            notes: req.notes,
        }).await?;
    Ok(Json(AmmoTypeResponse { entry: t }))
}

async fn delete_type(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteAmmoTypeUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
