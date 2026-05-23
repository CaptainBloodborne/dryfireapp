use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    domain::{
        repositories::user::{SessionRepository, UserRepository, VerificationRepository},
        services::{
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
    pub token_handler: Arc<dyn TokenHandler>,
    pub mailer: Arc<dyn Mailer>,

    pub user_repo: Arc<dyn UserRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    pub verification_repo: Arc<dyn VerificationRepository>,
}