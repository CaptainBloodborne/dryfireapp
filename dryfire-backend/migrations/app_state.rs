//! Shared, cheap-to-clone application state.
//!
//! Every field is either a primitive `Clone` or `Arc<...>`, so the
//! whole `AppState` clones in O(1). Handlers extract it via
//! `State<AppState>`.
//!
//! All services here are **trait objects**, not concrete types — that
//! way the controller and use-case layers depend only on the domain
//! traits, and tests can swap mock implementations in.

use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    domain::{
        repositories::{
            ballistics::BallisticProfileRepository,
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

    // primitives
    pub hasher: Arc<dyn Hasher>,
    pub signer: Arc<dyn Signer>,
    pub cipher: Arc<dyn FieldCipher>,
    pub token_handler: Arc<dyn TokenHandler>,
    pub mailer: Arc<dyn Mailer>,
    pub audit: Arc<dyn AuditLogger>,

    // user domain
    pub user_repo: Arc<dyn UserRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    pub verification_repo: Arc<dyn VerificationRepository>,

    // ballistics / scope domains
    pub ballistic_profile_repo: Arc<dyn BallisticProfileRepository>,
    pub scope_profile_repo: Arc<dyn ScopeProfileRepository>,
}
