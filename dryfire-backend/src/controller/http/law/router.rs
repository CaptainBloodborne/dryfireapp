// src/controller/http/law/router.rs

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
            LawUpdatesSinceUseCase, ListLawsUseCase, SearchLawsUseCase,
            UpsertLawUseCase,
        },
    },
    controller::http::{
        armory::payload::{Page, PageRequest},
        errors::{ApiError, ApiResult},
        law::payload::*,
        middleware::auth::{AuthUser, require_auth},
    },
    domain::{
        entities::law::{Law, LawTag},
        errors::DomainError,
        repositories::armory::PageQuery,
    },
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(upsert))
        .route("/search", get(search))
        .route("/updates", get(updates_since))
        .route("/{id}", get(get_one))
        .layer(middleware::from_fn_with_state(state, require_auth))
}

async fn list(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(q): Query<ListLawsQuery>,
) -> ApiResult<Json<Page<LawSummary>>> {
    let tags = q.parse_tags()?;
    let page = q.page;
    let size = q.size;
    let pq = PageQuery {
        page, size, sort: None, filter_text: None,
    };
    let (items, total) = ListLawsUseCase::new(&state)
        .execute(q.region.as_deref(), &tags, pq)
        .await?;
    Ok(Json(Page {
        items: items.iter().map(LawSummary::from).collect(),
        total, page, size,
    }))
}

async fn get_one(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LawFull>> {
    let law = state.law_repo.find_by_id(id).await?
        .ok_or(DomainError::LawNotFound)?;
    Ok(Json(LawFull::from(&law)))
}

async fn search(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(q): Query<SearchLawsQuery>,
) -> ApiResult<Json<Page<LawSummary>>> {
    if q.q.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "VALIDATION_QUERY",
            "query is empty",
        ));
    }
    let page = q.page;
    let size = q.size;
    let pq = PageQuery { page, size, sort: None, filter_text: None };
    let (items, total) = SearchLawsUseCase::new(&state)
        .execute(q.region.as_deref(), &q.q, pq)
        .await?;
    Ok(Json(Page {
        items: items.iter().map(LawSummary::from).collect(),
        total, page, size,
    }))
}

async fn updates_since(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Query(q): Query<UpdatesSinceQuery>,
) -> ApiResult<Json<Vec<LawSummary>>> {
    let items = LawUpdatesSinceUseCase::new(&state)
        .execute(q.region.as_deref(), q.since)
        .await?;
    Ok(Json(items.iter().map(LawSummary::from).collect()))
}

/// Admin-only — for now we only check that the caller is authenticated;
/// extend with a role check (`auth.0.is_admin()`) once you add that field.
async fn upsert(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Json(req): Json<UpsertLawRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let tags: Vec<LawTag> = req.tags.iter()
        .map(|s| s.parse::<LawTag>())
        .collect::<Result<_, _>>()?;
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let law = Law {
        id,
        region: req.region,
        slug: req.slug,
        title: req.title,
        body: req.body,
        version: req.version,
        tags,
        published_at: req.published_at.unwrap_or(now),
        updated_at: now,
    };
    UpsertLawUseCase::new(&state).execute(law).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}