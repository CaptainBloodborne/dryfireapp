//! Armory HTTP routes.
//!
//! Three composed subtrees mounted at `/api/v1/armory`:
//!
//! 1. `GET /catalog`, `GET /catalog/{id}` — public-ish (auth required
//!    so we know who is reading, but no role check).
//! 2. `POST/PATCH/DELETE /catalog/...` — admin only.
//! 3. `*/guns/*` — user CRUD; ownership enforced in the repo.

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
            CreateCatalogEntryUseCase, CreateCatalogInput,
            DeleteCatalogEntryUseCase, DeleteGunUseCase, GetCatalogEntryUseCase,
            GetGunUseCase, ListCatalogUseCase, ListGunsUseCase,
            RegisterGunInput, RegisterGunUseCase, UpdateCatalogEntryUseCase,
            UpdateCatalogInput, UpdateGunInput, UpdateGunUseCase,
        },
    },
    controller::http::{
        armory::payload::*,
        errors::ApiResult,
        middleware::auth::{AuthUser, require_admin, require_auth},
    },
    domain::repositories::armory::{CatalogFilter, GunFilter},
    utils::paging::{Page, PageQuery},
};

// Tiny adapter — converts an HTTP DTO to the use-case input. Kept in
// the router (controller) layer because the use-case shouldn't know
// about the wire format.
pub mod adapters {
    use super::*;
    use crate::application::armory_use_cases::RegisterGunInput;

    pub fn create_to_input(user_id: Uuid, req: CreateGunRequest) -> RegisterGunInput {
        RegisterGunInput {
            user_id,
            catalog_id: req.catalog_id,
            manufacturer: req.manufacturer,
            model: req.model,
            class: req.class,
            caliber: req.caliber,
            serial: req.serial,
            date_of_purchase: req.date_of_purchase,
            photo_url: req.photo_url,
            notes: req.notes,
        }
    }
    pub fn update_to_input(req: UpdateGunRequest) -> UpdateGunInput {
        UpdateGunInput {
            catalog_id: req.catalog_id,
            manufacturer: req.manufacturer,
            model: req.model,
            class: req.class,
            caliber: req.caliber,
            serial: req.serial,
            date_of_purchase: req.date_of_purchase,
            photo_url: req.photo_url,
            notes: req.notes,
        }
    }
    pub fn cat_create_to_input(req: CreateCatalogRequest) -> CreateCatalogInput {
        CreateCatalogInput {
            manufacturer: req.manufacturer, model: req.model,
            class: req.class, caliber: req.caliber,
            barrel_length_mm: req.barrel_length_mm,
            weight_g: req.weight_g, capacity: req.capacity,
            notes: req.notes,
        }
    }
    pub fn cat_update_to_input(req: UpdateCatalogRequest) -> UpdateCatalogInput {
        UpdateCatalogInput {
            manufacturer: req.manufacturer, model: req.model,
            class: req.class, caliber: req.caliber,
            barrel_length_mm: req.barrel_length_mm,
            weight_g: req.weight_g, capacity: req.capacity,
            notes: req.notes,
        }
    }
}

pub fn routes(state: AppState) -> Router<AppState> {
    let user_subtree = Router::new()
        .route("/guns", get(list_guns).post(create_gun))
        .route(
            "/guns/{id}",
            get(get_gun).patch(update_gun).delete(delete_gun),
        )
        .route("/guns/{id}/serial", get(get_gun_serial))
        .route("/catalog", get(list_catalog))
        .route("/catalog/{id}", get(get_catalog_entry))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let admin_subtree = Router::new()
        .route("/catalog", post(create_catalog_entry))
        .route(
            "/catalog/{id}",
            axum::routing::patch(update_catalog_entry)
                .delete(delete_catalog_entry),
        )
        .layer(middleware::from_fn_with_state(state, require_admin));

    Router::new()
        .merge(user_subtree)
        .nest("/admin", admin_subtree)
}

// =================== gun handlers =================== //

async fn create_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateGunRequest>,
) -> ApiResult<(StatusCode, Json<GunResponse>)> {
    let gun = RegisterGunUseCase { state: &state }
        .execute(adapters::create_to_input(auth.0.id(), req))
        .await?;
    Ok((StatusCode::CREATED, Json(GunResponse::from(&gun))))
}

async fn list_guns(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(f): Query<GunListQuery>,
) -> ApiResult<Json<Page<GunResponse>>> {
    let p = page.normalized();
    let filter = GunFilter { class: f.class, caliber: f.caliber, q: f.q };
    let sort_field = p.sort.as_ref().map(|s| s.field.as_str());
    let (items, total) = ListGunsUseCase { state: &state }
        .execute(auth.0.id(), &filter, p.limit(), p.offset(), sort_field).await?;
    let resp: Vec<GunResponse> = items.iter().map(GunResponse::from).collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn get_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GunResponse>> {
    let g = GetGunUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(Json(GunResponse::from(&g)))
}

async fn get_gun_serial(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GunSerialResponse>> {
    let g = GetGunUseCase { state: &state }.execute(auth.0.id(), id).await?;
    // Audit: revealing the serial is sensitive.
    state.audit.record(
        crate::domain::services::audit::AuditEntry::new("gun.serial_reveal")
            .user(auth.0.id())
            .resource("gun", id),
    ).await;
    Ok(Json(GunSerialResponse::from_gun(&g)))
}

async fn update_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateGunRequest>,
) -> ApiResult<Json<GunResponse>> {
    let g = UpdateGunUseCase { state: &state }
        .execute(auth.0.id(), id, adapters::update_to_input(req)).await?;
    Ok(Json(GunResponse::from(&g)))
}

async fn delete_gun(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteGunUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// =================== catalog handlers =================== //

async fn list_catalog(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(f): Query<CatalogListQuery>,
) -> ApiResult<Json<Page<CatalogResponse>>> {
    let p = page.normalized();
    let filter = CatalogFilter { class: f.class, caliber: f.caliber, q: f.q };
    let (items, total) = ListCatalogUseCase { state: &state }
        .execute(&filter, p.limit(), p.offset()).await?;
    let resp: Vec<CatalogResponse> = items.into_iter().map(|entry| CatalogResponse { entry }).collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn get_catalog_entry(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CatalogResponse>> {
    let e = GetCatalogEntryUseCase { state: &state }.execute(id).await?;
    Ok(Json(CatalogResponse { entry: e }))
}

// ---- admin ---- //

async fn create_catalog_entry(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateCatalogRequest>,
) -> ApiResult<(StatusCode, Json<CatalogResponse>)> {
    let e = CreateCatalogEntryUseCase { state: &state }
        .execute(auth.0.id(), adapters::cat_create_to_input(req)).await?;
    Ok((StatusCode::CREATED, Json(CatalogResponse { entry: e })))
}

async fn update_catalog_entry(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCatalogRequest>,
) -> ApiResult<Json<CatalogResponse>> {
    let e = UpdateCatalogEntryUseCase { state: &state }
        .execute(auth.0.id(), id, adapters::cat_update_to_input(req)).await?;
    Ok(Json(CatalogResponse { entry: e }))
}

async fn delete_catalog_entry(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteCatalogEntryUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
