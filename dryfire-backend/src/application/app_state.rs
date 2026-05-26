use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    domain::{
        repositories::{
            ammo::AmmoRepository,
            armory::{GunCatalogRepository, GunRepository},
            ballistics::BallisticProfileRepository,
            law::LawRepository,
            license::{LicenseNotificationRepository, LicenseRepository},
            scope::ScopeProfileRepository,
            user::{SessionRepository, UserRepository, VerificationRepository},
        },
        services::{
            audit::AuditLogger,
            cipher::FieldCipher,
            crypto::{Hasher, Signer},
            identity::TokenHandler,
            mail::Mailer,
        },
    },
    infra::config::Config,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,

    pub hasher: Arc<dyn Hasher>,
    pub signer: Arc<dyn Signer>,
    pub cipher: Arc<dyn FieldCipher>,
    pub token_handler: Arc<dyn TokenHandler>,
    pub mailer: Arc<dyn Mailer>,
    pub audit: Arc<dyn AuditLogger>,

    // user
    pub user_repo: Arc<dyn UserRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    pub verification_repo: Arc<dyn VerificationRepository>,

    // armory domain
    pub gun_repo: Arc<dyn GunRepository>,
    pub gun_catalog_repo: Arc<dyn GunCatalogRepository>,

    // ammo domain
    pub ammo_repo: Arc<dyn AmmoRepository>,

    // licenses domain
    pub license_repo: Arc<dyn LicenseRepository>,
    pub license_notification_repo: Arc<dyn LicenseNotificationRepository>,

    // ballistics
    pub ballistic_profile_repo: Arc<dyn BallisticProfileRepository>,

    // scope
    pub scope_profile_repo: Arc<dyn ScopeProfileRepository>,

    // laws domain
    pub law_repo: Arc<dyn LawRepository>,
}
