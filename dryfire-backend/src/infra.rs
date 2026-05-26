use crate::{
    application::app_state::AppState,
    controller::http::AxumServer,
    infra::{
        config::init_config,
        db::pool::init_pool,
        hash::aes_gcm_cipher::AesGcmCipher,
        hash::argon::ArgonHasher,
        hash::hmac_signer::HmacSigner,
        mail::logging::LoggingMailer,
        server::Server,
    },
    utils::tokengenerator::TokenGenerator,
};
use std::{sync::Arc, time::Duration};

pub mod config;
pub mod db;
pub mod hash;
pub mod mail;
pub mod scheduler;
pub mod server;

/// Bootstrap: read config, connect to DB, run migrations,
/// build [`AppState`], hand off to the HTTP server.
pub async fn init_app() -> anyhow::Result<()> {
    let config = Arc::new(init_config()?);
    tracing::info!(?config.controller, "config loaded");

    let pool = init_pool(&config).await?;
    tracing::info!("DB pool ready, running migrations");
    sqlx::migrate!("./migrations").run(&pool).await?;

    // ---- wire up primitives (domain trait - infra impl) ---- //
    let hasher = Arc::new(ArgonHasher) as Arc<dyn crate::domain::services::crypto::Hasher>;
    let signer = Arc::new(HmacSigner::new(config.token_secret.as_bytes()))
        as Arc<dyn crate::domain::services::crypto::Signer>;
    let token_handler = Arc::new(TokenGenerator::new(signer.clone()))
        as Arc<dyn crate::domain::services::identity::TokenHandler>;

    // 32-byte field encryption key. We accept a hex-encoded value in
    // the env so it's easy to paste from `openssl rand -hex 32`.
    let field_key = hex::decode(&config.field_encryption_key_hex)
        .map_err(|e| anyhow::anyhow!("FIELD_ENCRYPTION_KEY_HEX is not valid hex: {e}"))?;
    let cipher = Arc::new(AesGcmCipher::from_key_bytes(&field_key)?)
        as Arc<dyn crate::domain::services::cipher::FieldCipher>;

    let mailer = Arc::new(LoggingMailer) as Arc<dyn crate::domain::services::mail::Mailer>;
    let audit = Arc::new(crate::infra::db::audit_log::PgAuditLogger::new(pool.clone()))
        as Arc<dyn crate::domain::services::audit::AuditLogger>;

    // ---- wire up repositories ---- //
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
    let ballistic_profile_repo = Arc::new(
        crate::infra::db::ballistic_profile_repo::PgBallisticProfileRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::ballistics::BallisticProfileRepository>;
    let scope_profile_repo = Arc::new(
        crate::infra::db::scope_profile_repo::PgScopeProfileRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::scope::ScopeProfileRepository>;
    let gun_repo = Arc::new(
        crate::infra::db::gun_repo::PgGunRepository::new(pool.clone(), cipher.clone()),
    )
        as Arc<dyn crate::domain::repositories::armory::GunRepository>;
    let gun_catalog_repo = Arc::new(
        crate::infra::db::gun_catalog_repo::PgGunCatalogRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::armory::GunCatalogRepository>;
    let license_repo = Arc::new(
        crate::infra::db::license_repo::PgLicenseRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::license::LicenseRepository>;
    let license_notification_repo = Arc::new(
        crate::infra::db::license_notification_repo::PgLicenseNotificationRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::license::LicenseNotificationRepository>;
    let ammo_repo = Arc::new(
        crate::infra::db::ammo_repo::PgAmmoRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::ammo::AmmoRepository>;
    let law_repo = Arc::new(
        crate::infra::db::law_repo::PgLawRepository::new(pool.clone()),
    )
        as Arc<dyn crate::domain::repositories::law::LawRepository>;

    let state = AppState {
        pool,
        config: config.clone(),
        hasher,
        signer,
        cipher,
        token_handler,
        mailer,
        audit,
        user_repo,
        session_repo,
        verification_repo,
        ballistic_profile_repo,
        scope_profile_repo,
        gun_repo,
        gun_catalog_repo,
        license_repo,
        license_notification_repo,
        ammo_repo,
        law_repo,
    };

    // Spawn the license-reminder scheduler. Detached: it lives for the
    // process lifetime and shuts down when its DB calls start erroring
    // (which happens when the main task drops the pool on Ctrl-C).
    let scheduler_state = state.clone();
    let tick = Duration::from_secs(config.scheduler_tick_secs);
    tokio::spawn(async move {
        crate::infra::scheduler::license_reminders::run(scheduler_state, tick).await;
    });

    let server = AxumServer::new("dryfire-backend", config)?;
    server.start_server(state).await?;
    Ok(())
}
