pub mod errors;
pub mod middleware;
pub mod user;

use async_trait::async_trait;
use axum::{Router, http::StatusCode, response::IntoResponse, routing::get, serve};

use crate::{application::app_state::AppState, infra::{config::{Config, init_config}, server::Server}};

pub struct AxumServer {
    pub app_name: String,
    pub config: Config,
}

impl AxumServer {
    pub fn new(app_name: &str) -> anyhow::Result<Self> {
        let app_config = init_config()?;
        anyhow::Ok(Self { app_name: app_name.to_string(), config: app_config })
    }

}
#[async_trait]
impl Server for AxumServer {
    async fn start_server(&self, state: AppState) -> anyhow::Result<()> {
        let router: Router = Router::new()
            .route("/health", get(healthcheck))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

        serve(listener, router).await?;

        anyhow::Ok(())
    }

    fn shutdown(&self) -> anyhow::Result<()> {
        anyhow::Ok(())
    }
}

pub async fn healthcheck() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}