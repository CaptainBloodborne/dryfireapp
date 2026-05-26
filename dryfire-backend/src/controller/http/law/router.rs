//! Law HTTP routes.
//!
//! Two subtrees mounted at `/api/v1/laws`:
//!
//! 1. Public (`require_auth`):
//!      GET /laws            — list / filter
//!      GET /laws/search?q=  — full-text search
//!      GET /laws/changes    — what changed since user's last visit
//!      GET /laws/{id}       — get one (by uuid)
//!      GET /laws/by-key/{key} — get one (by stable key)
//!      GET /laws/{id}/versions — revision history
//!      GET /categories      — taxonomy
//!
//! 2. Admin (`require_admin`):
//!      POST /admin/laws, PATCH /admin/laws (by law_key), DELETE /admin/laws/{id}
//!      POST/PATCH/DELETE /admin/categories
//!
//! The PATCH on laws is keyed by `law_key` (in the body), not by id in
//! the URL — that's the natural unit for an ingester that reads
//! upstream feeds keyed by external slug.

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
        law_use_cases::{
            ChangesSinceUseCase, CreateCategoryInput, CreateCategoryUseCase,
            CreateLawInput, CreateLawUseCase, DeleteCategoryUseCase,
            DeleteLawUseCase, GetLawByKeyUseCase, GetLawUseCase,
            LawVersionsUseCase, ListCategoriesUseCase, ListLawsUseCase,
            SearchLawsUseCase, UpdateCategoryInput, UpdateCategoryUseCase,
            UpdateLawByKeyInput, UpdateLawByKeyUseCase,
        },
    },
    controller::http::{
        errors::ApiResult,
        law::payload::*,
        middleware::auth::{AuthUser, require_admin, require_auth},
    },
    domain::repositories::law::LawFilter,
    utils::paging::{Page, PageQuery},
};

pub fn routes(state: AppState) -> Router<AppState> {
    let public = Router::new()
        // Static segments first so they win against `/laws/{id}`.
        .route("/laws/search",   get(search))
        .route("/laws/changes",  get(changes))
        .route("/laws/by-key/{key}", get(by_key))
        .route("/laws",          get(list_laws))
        .route("/laws/{id}",     get(get_law))
        .route("/laws/{id}/versions", get(versions))
        .route("/categories",    get(list_categories))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let admin = Router::new()
        .route("/laws", post(create_law).patch(update_law))
        .route("/laws/{id}", axum::routing::delete(delete_law))
        .route("/categories", post(create_category))
        .route("/categories/{id}",
               axum::routing::patch(update_category).delete(delete_category))
        .layer(middleware::from_fn_with_state(state, require_admin));

    Router::new()
        .merge(public)
        .nest("/admin", admin)
}

// public handlers

async fn list_laws(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(f): Query<LawListQuery>,
) -> ApiResult<Json<Page<LawResponse>>> {
    let p = page.normalized();
    let filter = LawFilter {
        region: f.region,
        category_id: f.category_id,
        any_tags: parse_tags_csv(f.any_tags.as_deref()),
        all_tags: parse_tags_csv(f.all_tags.as_deref()),
        updated_after: f.updated_after,
        effective_after: f.effective_after,
    };
    let (items, total) = ListLawsUseCase { state: &state }
        .execute(&filter, p.limit(), p.offset()).await?;
    let resp: Vec<LawResponse> = items.into_iter()
        .map(|law| LawResponse { law })
        .collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn search(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<Page<LawSearchHitResponse>>> {
    let p = page.normalized();
    let filter = LawFilter {
        region: q.region,
        category_id: q.category_id,
        any_tags: parse_tags_csv(q.any_tags.as_deref()),
        all_tags: parse_tags_csv(q.all_tags.as_deref()),
        updated_after: None,
        effective_after: None,
    };
    let (items, total) = SearchLawsUseCase { state: &state }
        .execute(&q.q, &filter, p.limit(), p.offset()).await?;
    let resp: Vec<LawSearchHitResponse> = items.into_iter()
        .map(|hit| LawSearchHitResponse { hit })
        .collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn changes(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(page): Query<PageQuery>,
    Query(q): Query<ChangesSinceQuery>,
) -> ApiResult<Json<Page<LawResponse>>> {
    let p = page.normalized();

    // Use explicit `since` if supplied, otherwise the user's last_visit_at,
    // and finally fall back to account creation if neither exists.
    let since = q.since
        .or_else(|| auth.0.last_visit_at())
        .unwrap_or_else(|| auth.0.created_at());
    let region = q.region.unwrap_or_else(|| auth.0.region().as_str().to_string());

    let (items, total) = ChangesSinceUseCase { state: &state }
        .execute(&region, since, p.limit(), p.offset()).await?;
    let resp: Vec<LawResponse> = items.into_iter()
        .map(|law| LawResponse { law })
        .collect();
    Ok(Json(Page::new(resp, total, &p)))
}

async fn get_law(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LawResponse>> {
    let l = GetLawUseCase { state: &state }.execute(id).await?;
    Ok(Json(LawResponse { law: l }))
}

async fn by_key(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(key): Path<String>,
) -> ApiResult<Json<LawResponse>> {
    let l = GetLawByKeyUseCase { state: &state }.execute(&key).await?;
    Ok(Json(LawResponse { law: l }))
}

async fn versions(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<LawVersionResponse>>> {
    let items = LawVersionsUseCase { state: &state }.execute(id).await?;
    Ok(Json(items.into_iter().map(|version| LawVersionResponse { version }).collect()))
}

async fn list_categories(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
) -> ApiResult<Json<Vec<CategoryResponse>>> {
    let items = ListCategoriesUseCase { state: &state }.execute().await?;
    Ok(Json(items.into_iter().map(|category| CategoryResponse { category }).collect()))
}

// admin handlers

async fn create_law(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateLawRequest>,
) -> ApiResult<(StatusCode, Json<LawResponse>)> {
    let l = CreateLawUseCase { state: &state }
        .execute(auth.0.id(), CreateLawInput {
            law_key: req.law_key,
            title: req.title,
            summary: req.summary,
            body: req.body,
            region: req.region,
            category_id: req.category_id,
            tags: req.tags,
            effective_at: req.effective_at,
        }).await?;
    Ok((StatusCode::CREATED, Json(LawResponse { law: l })))
}

async fn update_law(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<UpdateLawRequest>,
) -> ApiResult<Json<LawResponse>> {
    let l = UpdateLawByKeyUseCase { state: &state }
        .execute(auth.0.id(), UpdateLawByKeyInput {
            law_key: req.law_key,
            title: req.title,
            summary: req.summary,
            body: req.body,
            region: req.region,
            category_id: req.category_id,
            tags: req.tags,
            effective_at: req.effective_at,
        }).await?;
    Ok(Json(LawResponse { law: l }))
}

async fn delete_law(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteLawUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateCategoryRequest>,
) -> ApiResult<(StatusCode, Json<CategoryResponse>)> {
    let c = CreateCategoryUseCase { state: &state }
        .execute(auth.0.id(), CreateCategoryInput {
            code: req.code, name: req.name,
            parent_id: req.parent_id, sort_order: req.sort_order,
        }).await?;
    Ok((StatusCode::CREATED, Json(CategoryResponse { category: c })))
}

async fn update_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategoryRequest>,
) -> ApiResult<Json<CategoryResponse>> {
    let c = UpdateCategoryUseCase { state: &state }
        .execute(auth.0.id(), id, UpdateCategoryInput {
            code: req.code, name: req.name,
            parent_id: req.parent_id, sort_order: req.sort_order,
        }).await?;
    Ok(Json(CategoryResponse { category: c }))
}

async fn delete_category(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    DeleteCategoryUseCase { state: &state }.execute(auth.0.id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
