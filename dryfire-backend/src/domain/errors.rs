//! Domain-layer error types. These are pure: they don't know about HTTP
//! status codes, SQL, etc. The HTTP controller maps them to responses.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("login must be 3..=64 chars, ASCII letters/digits/underscore only")]
    Login,

    #[error("email is not a valid address")]
    Email,

    #[error("password is too short (min 8 chars)")]
    PasswordTooShort,
    #[error("password is too long (max 128 chars)")]
    PasswordTooLong,
    #[error("password must contain at least one digit")]
    PasswordNoDigit,
    #[error("password must contain at least one uppercase letter")]
    PasswordNoUppercase,
    #[error("password must contain at least one lowercase letter")]
    PasswordNoLowercase,
    #[error("password must contain at least one special character")]
    PasswordNoSpecial,

    #[error("invalid region code `{0}` — expected ISO 3166-1 alpha-2")]
    Region(String),
    #[error("unsupported language `{0}`")]
    Language(String),

    #[error("validation failed: {0}")]
    Custom(String),
}

#[derive(Debug, Error)]
pub enum DomainError {
    // user-domain
    #[error("user with this email already exists")]
    EmailAlreadyExists,
    #[error("user with this login already exists")]
    LoginAlreadyTaken,
    #[error("user is underage (required {required}, got {actual})")]
    Underage { required: i32, actual: i32 },
    #[error("user not found")]
    UserNotFound,
    #[error("user is not verified")]
    NotVerified,
    #[error("user already verified")]
    AlreadyVerified,
    #[error("user is blocked")]
    Blocked,

    // armory
    #[error("gun not found")]
    GunNotFound,
    #[error("gun with this serial already exists")]
    GunSerialAlreadyExists,
    #[error("ammo lot not found")]
    AmmoLotNotFound,
    #[error("not enough ammo on hand (have {have}, need {need})")]
    AmmoInsufficient { have: i64, need: i64 },

    // license
    #[error("license not found")]
    LicenseNotFound,
    #[error("license already expired")]
    LicenseExpired,

    // ballistics
    #[error("ballistic profile not found")]
    BallisticProfileNotFound,
    #[error("invalid ballistic input: {0}")]
    BallisticInput(String),

    // scope
    #[error("scope profile not found")]
    ScopeProfileNotFound,
    #[error("requested adjustment exceeds scope range")]
    ScopeRangeExceeded,

    // law
    #[error("law not found")]
    LawNotFound,

    // generic ownership
    #[error("forbidden: resource not owned by current user")]
    NotOwner,

    // auth
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("token is invalid or malformed")]
    InvalidToken,
    #[error("token expired")]
    TokenExpired,
    #[error("session not found or revoked")]
    SessionRevoked,
    #[error("verification token invalid or expired")]
    InvalidVerificationToken,
    #[error("password reset token invalid or expired")]
    InvalidResetToken,

    // validation
    #[error(transparent)]
    Validation(#[from] ValidationError),

    // catch-all
    #[error("infrastructure error: {0}")]
    Infra(#[source] anyhow::Error),
}

impl From<sqlx::Error> for DomainError {
    fn from(e: sqlx::Error) -> Self {
        DomainError::Infra(e.into())
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
