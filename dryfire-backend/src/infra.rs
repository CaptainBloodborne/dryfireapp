use crate::{
    application::app_state::AppState,
    controller::http::AxumServer,
    infra::{
        config::init_config,
        db::pool::init_pool,
        hash::argon::ArgonHasher,
        hash::hmac_signer::HmacSigner,
        mail::logging::LoggingMailer,
        server::Server,
    },
    utils::tokengenerator::TokenGenerator,
};
use std::sync::Arc;

pub mod config;
pub mod db;
pub mod hash;
pub mod mail;
pub mod server;

/// Bootstrap: read config, connect to DB, run migrations,
/// build [`AppState`], hand off to the HTTP server.
pub async fn init_app() -> anyhow::Result<()> {
    let config = Arc::new(init_config()?);
    tracing::info!(?config.controller, "config loaded");

    let pool = init_pool(&config).await?;
    tracing::info!("DB pool ready, running migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;

    // ---- wire up the services (domain trait → infra impl) ---- //
    let hasher = Arc::new(ArgonHasher) as Arc<dyn crate::domain::services::crypto::Hasher>;
    let signer = Arc::new(HmacSigner::new(config.token_secret.as_bytes()))
        as Arc<dyn crate::domain::services::crypto::Signer>;
    let token_handler = Arc::new(TokenGenerator::new(signer.clone()))
        as Arc<dyn crate::domain::services::identity::TokenHandler>;

    let user_repo = Arc::new(crate::infra::db::user_repo::PgUserRepository::new(
        pool.clone(),
    ))
        as Arc<dyn crate::domain::repositories::user::UserRepository>;
    let session_repo = Arc::new(
        crate::infra::db::session_repo::PgSessionRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::user::SessionRepository>;
    let verification_repo = Arc::new(
        crate::infra::db::verification_repo::PgVerificationRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::user::VerificationRepository>;

    let mailer = Arc::new(LoggingMailer) as Arc<dyn crate::domain::services::mail::Mailer>;

    let state = AppState {
        pool,
        config: config.clone(),
        hasher,
        signer,
        token_handler,
        user_repo,
        session_repo,
        verification_repo,
        mailer,
    };

    let server = AxumServer::new("dryfire-backend", config)?;
    server.start_server(state).await?;
    Ok(())
}
