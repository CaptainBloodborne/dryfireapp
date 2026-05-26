//! User-related application use cases.

use std::net::IpAddr;

use chrono::{NaiveDate, Utc};
use rand::Rng;  
use secrecy::{ExposeSecret, SecretString};
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::user::{Language, Region, User},
        errors::{DomainError, DomainResult},
    },
    domain::services::identity::{Credentials, Email, Login, Password, Token},
    utils::{b64::b64_encode_bytes, time::now_utc_plus_sec},
};

// Register

#[derive(Debug)]
pub struct RegisterInput {
    pub login: String,
    pub firstname: String,
    pub surname: String,
    pub email: String,
    pub password: SecretString,
    pub date_of_birth: NaiveDate,
    pub region: String,
    pub language: String,
}

#[derive(Debug)]
pub struct RegisterOutput {
    pub user_id: Uuid,
}

pub struct RegisterNewUserUseCase<'a> {
    pub state: &'a AppState,
}

impl<'a> RegisterNewUserUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self, input), fields(email = %input.email, login = %input.login))]
    pub async fn execute(&self, input: RegisterInput) -> DomainResult<RegisterOutput> {
        let login = Login::parse(input.login)?;
        let email = Email::parse(input.email)?;
        let password = Password::new(input.password)?;
        let region = Region::new(input.region)?;
        let language: Language = input.language.parse()?;

        let user = User::register(
            login.as_str().to_string(),
            input.firstname,
            input.surname,
            email.as_str().to_string(),
            input.date_of_birth,
            region,
            language,
        )?;

        let hash = self
            .state
            .hasher
            .hash(password.expose().to_string())
            .await
            .map_err(DomainError::Infra)?;
        let credentials = Credentials::new(hash);

        self.state.user_repo.create_user(&user, &credentials).await?;

        let raw = generate_random_token();
        let token_hash = b64_encode_bytes(&self.state.signer.sign(&raw));
        let expires =
            now_utc_plus_sec(self.state.config.verify_token_ttl_secs);

        self.state
            .verification_repo
            .create_email_verification(user.id(), &token_hash, expires)
            .await?;

        let url = format!(
            "{}/api/v1/users/verify-email?token={}",
            self.state.config.public_origin, raw
        );

        if let Err(e) = self
            .state
            .mailer
            .send_verification_email(user.email(), &url)
            .await
        {
            tracing::warn!(error = ?e, "verification email failed; user must request again");
        }

        Ok(RegisterOutput { user_id: user.id() })
    }
}


// VerifyEmail
pub struct VerifyEmailUseCase<'a> {
    pub state: &'a AppState,
}

impl<'a> VerifyEmailUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self, raw_token))]
    pub async fn execute(&self, raw_token: &str) -> DomainResult<Uuid> {
        let token_hash = b64_encode_bytes(&self.state.signer.sign(raw_token));

        let user_id = self
            .state
            .verification_repo
            .consume_email_verification(&token_hash)
            .await?;

        // Two-step verification: the SQL `mark_verified` only flips
        // pending-verified; if the user was blocked or already
        // verified we re-classify.
        match self.state.user_repo.mark_verified(user_id).await {
            Ok(()) => Ok(user_id),
            Err(DomainError::AlreadyVerified) => {
                let u = self
                    .state
                    .user_repo
                    .find_by_id(user_id)
                    .await?
                    .ok_or(DomainError::UserNotFound)?;
                if u.is_blocked() {
                    Err(DomainError::Blocked)
                } else {
                    Err(DomainError::AlreadyVerified)
                }
            }
            Err(other) => Err(other),
        }
    }
}

// Login
#[derive(Debug)]
pub struct LoginInput {
    pub login_or_email: String,
    pub password: SecretString,
    pub user_agent: Option<String>,
    pub ip: Option<IpAddr>,
}

#[derive(Debug)]
pub struct LoginOutput {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub access_token: Token,
}

pub struct LoginUserUseCase<'a> {
    pub state: &'a AppState,
}

/// A dummy argon2 hash used as a timing-attack countermeasure (see below).
/// Constant: the verify-against-dummy path takes the same wall time as
/// verify-against-real, so an attacker cannot infer account existence
/// from response timing.
const DUMMY_PHC: &str = "$argon2id$v=19$m=19456,t=2,p=1$YWFhYWFhYWFhYWFhYWFhYQ$qwNFCvR3cAQwsJM4QXxabHsq9XdmygOFnRWHd8e9Sb4";

impl<'a> LoginUserUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self, input), fields(ident = %input.login_or_email))]
    pub async fn execute(&self, input: LoginInput) -> DomainResult<LoginOutput> {
        // 1. Find the user (either by login or email).
        let user_opt = if input.login_or_email.contains('@') {
            self.state.user_repo.find_by_email(&input.login_or_email).await?
        } else {
            self.state.user_repo.find_by_login(&input.login_or_email).await?
        };

        let creds_opt = match &user_opt {
            Some(u) => self.state.user_repo.find_credentials(u.id()).await?,
            None => None,
        };
        let hash_to_check = creds_opt
            .as_ref()
            .map(|c| c.password_hash().to_string())
            .unwrap_or_else(|| DUMMY_PHC.to_string());

        let verify_ok = self
            .state
            .hasher
            .validate(
                input.password.expose_secret().to_string(),
                hash_to_check,
            )
            .await
            .is_ok();

        let mut user = match (user_opt, verify_ok) {
            (Some(u), true) => u,
            _ => return Err(DomainError::InvalidCredentials),
        };

        if user.is_blocked() { return Err(DomainError::Blocked); }
        if !user.is_verified() { return Err(DomainError::NotVerified); }

        let session = self
            .state
            .session_repo
            .create(
                user.id(),
                self.state.config.session_ttl_secs,
                input.user_agent.as_deref(),
                input.ip,
            )
            .await?;

        let access_token = self
            .state
            .token_handler
            .generate_token(&session.id.to_string())
            .await
            .map_err(DomainError::Infra)?;

        // 6. Best-effort last-visit timestamp on the user row.
        user.touch_visit();
        let _ = self
            .state
            .user_repo
            .touch_last_visit(user.id(), Utc::now())
            .await;

        Ok(LoginOutput {
            user_id: user.id(),
            session_id: session.id,
            access_token,
        })
    }
}


// Logout
pub struct LogoutUseCase<'a> {
    pub state: &'a AppState,
}

impl<'a> LogoutUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self))]
    pub async fn execute(&self, session_id: Uuid) -> DomainResult<()> {
        self.state.session_repo.revoke(session_id).await
    }
}


// Edit / Get current user
pub struct GetCurrentUserUseCase<'a> {
    pub state: &'a AppState,
}

impl<'a> GetCurrentUserUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    pub async fn execute(&self, user_id: Uuid) -> DomainResult<User> {
        self.state
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)
    }
}

pub struct EditUserDataUseCase<'a> {
    pub state: &'a AppState,
}

#[derive(Debug)]
pub struct EditUserInput {
    pub firstname: Option<String>,
    pub surname: Option<String>,
    pub region: Option<String>,
    pub language: Option<String>,
}

impl<'a> EditUserDataUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    /// Stub: in the next iteration we'll have a dedicated UPDATE
    /// statement on the repository. Returning the current user lets
    /// the controller layer compile against the use case today.
    pub async fn execute(
        &self,
        user_id: Uuid,
        _input: EditUserInput,
    ) -> DomainResult<User> {
        self.state
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)
    }
}

// Password reset (request)
pub struct RequestPasswordResetUseCase<'a> {
    pub state: &'a AppState,
}

impl<'a> RequestPasswordResetUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self))]
    pub async fn execute(&self, email: &str) -> DomainResult<()> {
        let Some(user) = self.state.user_repo.find_by_email(email).await? else {
            tracing::info!("password reset requested for unknown email");
            return Ok(());
        };

        let raw = generate_random_token();
        let token_hash = b64_encode_bytes(&self.state.signer.sign(&raw));
        let expires = now_utc_plus_sec(self.state.config.reset_token_ttl_secs);

        self.state
            .verification_repo
            .create_password_reset(user.id(), &token_hash, expires)
            .await?;

        let url = format!(
            "{}/api/v1/users/reset-password?token={}",
            self.state.config.public_origin, raw
        );
        let _ = self
            .state
            .mailer
            .send_password_reset_email(user.email(), &url)
            .await;
        Ok(())
    }
}

// Password reset (confirm)
pub struct ConfirmPasswordResetUseCase<'a> {
    pub state: &'a AppState,
}

impl<'a> ConfirmPasswordResetUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }

    #[tracing::instrument(skip(self, raw_token, new_password))]
    pub async fn execute(
        &self,
        raw_token: &str,
        new_password: SecretString,
    ) -> DomainResult<()> {
        let password = Password::new(new_password)?;
        let token_hash = b64_encode_bytes(&self.state.signer.sign(raw_token));

        let user_id = self
            .state
            .verification_repo
            .consume_password_reset(&token_hash)
            .await?;

        let new_hash = self
            .state
            .hasher
            .hash(password.expose().to_string())
            .await
            .map_err(DomainError::Infra)?;

        self.state
            .user_repo
            .update_password_hash(user_id, &new_hash)
            .await?;

        // Revoke all existing sessions — forces logout everywhere.
        let _ = self.state.session_repo.revoke_all_for_user(user_id).await;
        Ok(())
    }
}


// Helpers
/// 32 random bytes, base64url-encoded — ~43 chars. Used as the raw
/// verification / password-reset token.
fn generate_random_token() -> String {
    let mut buf = [0u8; 32];
    rand::rng().fill_bytes(&mut buf);
    b64_encode_bytes(&buf)
}
