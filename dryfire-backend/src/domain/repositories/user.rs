use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::user::User,
    errors::{DomainError, DomainResult},
    services::identity::Credentials,
};

#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Atomically insert the user row and the credentials row.
    /// Implementations should detect unique-violations on `email`/`login`
    /// and return [`DomainError::EmailAlreadyExists`] /
    /// [`DomainError::LoginAlreadyTaken`].
    async fn create_user(
        &self,
        user: &User,
        credentials: &Credentials,
    ) -> DomainResult<()>;

    // lookups
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<User>>;
    async fn find_by_email(&self, email: &str) -> DomainResult<Option<User>>;
    async fn find_by_login(&self, login: &str) -> DomainResult<Option<User>>;
    async fn find_credentials(&self, user_id: Uuid)
        -> DomainResult<Option<Credentials>>;

    // existence checks-
    async fn email_exists(&self, email: &str) -> DomainResult<bool>;
    async fn login_exists(&self, login: &str) -> DomainResult<bool>;

    // mutations
    async fn mark_verified(&self, user_id: Uuid) -> DomainResult<()>;
    async fn update_password_hash(
        &self,
        user_id: Uuid,
        new_hash: &str,
    ) -> DomainResult<()>;
    async fn touch_last_visit(
        &self,
        user_id: Uuid,
        when: DateTime<Utc>,
    ) -> DomainResult<()>;
}

// ------------------------- session repo --------------------------- //

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.expires_at > now
    }
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(
        &self,
        user_id: Uuid,
        ttl_seconds: i64,
        user_agent: Option<&str>,
        ip: Option<std::net::IpAddr>,
    ) -> DomainResult<Session>;

    async fn find_active(&self, session_id: Uuid) -> DomainResult<Option<Session>>;
    async fn revoke(&self, session_id: Uuid) -> DomainResult<()>;
    async fn revoke_all_for_user(&self, user_id: Uuid) -> DomainResult<()>;
}

// --------------------- verification repo -------------------------- //

#[async_trait]
pub trait VerificationRepository: Send + Sync {
    /// Persist a verification token (we store *the HMAC of the token*,
    /// not the token itself). Returns the new row's id.
    async fn create_email_verification(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> DomainResult<Uuid>;

    /// Look up by token_hash. Mark it consumed atomically. Returns the
    /// `user_id` if a valid token was found, [`DomainError::InvalidVerificationToken`]
    /// otherwise.
    async fn consume_email_verification(
        &self,
        token_hash: &str,
    ) -> DomainResult<Uuid>;

    async fn create_password_reset(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> DomainResult<Uuid>;

    async fn consume_password_reset(
        &self,
        token_hash: &str,
    ) -> DomainResult<Uuid>;
}

// Helper for repositories that need to express ownership errors uniformly.
impl DomainError {
    pub fn not_found_to_option<T>(r: DomainResult<T>) -> DomainResult<Option<T>> {
        match r {
            Ok(v) => Ok(Some(v)),
            Err(DomainError::UserNotFound) => Ok(None),
            Err(other) => Err(other),
        }
    }
}
