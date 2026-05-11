use crate::{
    application::app_state::AppState, controller::http::AxumServer,
    infra::server::Server,
};

pub mod config;
pub mod db;
pub mod server;
pub mod hash;

pub async fn init_app() -> anyhow::Result<()> {
    let server = AxumServer::new("dryfire-backend")?;

    let state = AppState {};

    server.start_server(state).await?;

    anyhow::Ok(())
}
