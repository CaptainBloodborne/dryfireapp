//! HTTP entry point. Composes:
//!
//! - `/health` (unversioned)
//! - `/api/v1/users/...` (versioned, project requires URL versioning)
//! - Tracing, request ID, CORS, timeout, body-limit middleware
//! - Graceful shutdown on SIGTERM / Ctrl-C

pub mod errors;
pub mod middleware;
pub mod user;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
    middleware as ax_mw,
    response::IntoResponse,
    routing::get,
    serve,
};
use tower_http::{
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{
    application::app_state::AppState,
    controller::http::middleware::request_id::set_request_id,
    infra::{config::Config, server::Server},
};

pub struct AxumServer {
    pub app_name: String,
    pub config: Arc<Config>,
}

impl AxumServer {
    pub fn new(app_name: &str, config: Arc<Config>) -> anyhow::Result<Self> {
        Ok(Self {
            app_name: app_name.to_string(),
            config,
        })
    }
}

#[async_trait]
impl Server for AxumServer {
    async fn start_server(&self, state: AppState) -> anyhow::Result<()> {
        let router = build_router(state.clone());
        let addr = self.config.socket_addr()?;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!(app = %self.app_name, %addr, "server listening");

        serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await?;
        Ok(())
    }

    fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Compose the full app router. Kept as a free function so tests can
/// call it without standing up the [`AxumServer`] struct.
pub fn build_router(state: AppState) -> Router {
    // Public v1 routes (composition only — handlers themselves enforce auth).
    let v1_users = Router::new()
        .merge(user::router::public_routes())
        .merge(user::router::protected_routes(state.clone()));

    let v1 = Router::new().nest("/users", v1_users);

    Router::new()
        .route("/health", get(healthcheck))
        .nest("/api/v1", v1)
        .with_state(state)
        .layer(ax_mw::from_fn(set_request_id))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(RequestBodyLimitLayer::new(1024 * 1024 * 16)) // 16 MB
        .layer(permissive_cors())
}

fn permissive_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(Duration::from_secs(60 * 60))
}

pub async fn healthcheck() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// Wait for Ctrl-C (any OS) or SIGTERM (Unix). When the future resolves,
/// `axum::serve(...).with_graceful_shutdown(...)` stops accepting new
/// connections and drains in-flight requests.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Ctrl-C received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
