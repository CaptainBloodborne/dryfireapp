//! Request / response DTOs for the user routes. Kept separate from
//! domain entities so the wire format can evolve independently.

use chrono::{DateTime, NaiveDate, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::user::User;

// Register
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub login: String,
    pub firstname: String,
    pub surname: String,
    pub email: String,
    pub password: SecretString,
    pub date_of_birth: NaiveDate,
    pub region: String,
    pub language: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub message: &'static str,
}

// Login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Either the user's login or email.
    pub identifier: String,
    pub password: SecretString,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: Uuid,
    pub session_id: Uuid,
    /// Bearer token; client sends `Authorization: Bearer <token>` on
    /// subsequent calls.
    pub access_token: String,
}

// Verify email
#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyEmailResponse {
    pub user_id: Uuid,
    pub message: &'static str,
}

// Password reset
#[derive(Debug, Deserialize)]
pub struct RequestPasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmPasswordResetRequest {
    pub token: String,
    pub new_password: SecretString,
}

// Me
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub login: String,
    pub firstname: String,
    pub surname: String,
    pub email: String,
    pub date_of_birth: NaiveDate,
    pub region: String,
    pub language: String,
    pub status: String,
    pub last_visit_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<&User> for UserResponse {
    fn from(u: &User) -> Self {
        Self {
            id: u.id(),
            login: u.login().to_string(),
            firstname: u.firstname().to_string(),
            surname: u.surname().to_string(),
            email: u.email().to_string(),
            date_of_birth: u.date_of_birth(),
            region: u.region().as_str().to_string(),
            language: u.language().as_str().to_string(),
            status: u.status().as_str().to_string(),
            last_visit_at: u.last_visit_at(),
            created_at: u.created_at(),
        }
    }
}

// Edit user
#[derive(Debug, Deserialize)]
pub struct EditUserRequest {
    pub firstname: Option<String>,
    pub surname: Option<String>,
    pub region: Option<String>,
    pub language: Option<String>,
}
