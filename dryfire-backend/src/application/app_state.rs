use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    domain::{
        repositories::{
            user::{SessionRepository, UserRepository, VerificationRepository},
            armory::{AmmoRepository, GunRepository},
            ballistics::BallisticProfileRepository,
            law::LawRepository,
            license::LicenseRepository,
            scope::ScopeProfileRepository,
        },
        services::{
            ballistics::BallisticCalculator,
            crypto::{Hasher, Signer},
            identity::TokenHandler,
            mail::Mailer,
            scope::ScopeAdjuster,
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
    pub token_handler: Arc<dyn TokenHandler>,
    pub mailer: Arc<dyn Mailer>,

    // user
    pub user_repo: Arc<dyn UserRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    pub verification_repo: Arc<dyn VerificationRepository>,

    // armory
    pub gun_repo: Arc<dyn GunRepository>,
    pub ammo_repo: Arc<dyn AmmoRepository>,

    // license
    pub license_repo: Arc<dyn LicenseRepository>,

    // ballistics
    pub ballistic_profile_repo: Arc<dyn BallisticProfileRepository>,
    pub ballistic_calculator: Arc<dyn BallisticCalculator>,

    // scope
    pub scope_profile_repo: Arc<dyn ScopeProfileRepository>,
    pub scope_adjuster: Arc<dyn ScopeAdjuster>,

    // law
    pub law_repo: Arc<dyn LawRepository>,
}
