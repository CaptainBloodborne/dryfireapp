use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;

use crate::{domain::services::crypto::Hasher, utils::time::utc_now};

#[derive(Debug, thiserror::Error)]
pub enum PasswordValidationError {
    #[error("password must have at least 8 characters")]
    TooShort,

    #[error("password must have at most 128 characters")]
    TooLong,

    #[error("password must contain numbers")]
    NoNumbers,
    #[error("password must contain capital letters")]
    NoCapitalLetters,
    #[error("password must contain special characters")]
    NoSpecialCharacters,
}

type ValidationResult<T, E = PasswordValidationError> = Result<T, E>;

#[async_trait]
pub trait TokenHandler: Send + Sync {
    async fn generate_token<T: Hasher>(hasher: Arc<T>, ident: &str) -> anyhow::Result<Token>;
    async fn validate_token<T: Hasher>(
        hasher: Arc<T>,
        original_token: &Token,
    ) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct Token {
    pub ident: String,
    pub exp: String,
    pub sign_b64u: String,
}

pub struct Credentials {
    password_hash: String,
    last_visit: DateTime<Utc>,
}

//TODO: Make multiple regex patterns as array
impl Credentials {
    pub fn new(password_hash: String) -> Self {
        Self {
            password_hash,

            //
            last_visit: utc_now(),
        }
    }
}

pub struct Password(String);

impl Password {
    pub fn validate(&self) -> ValidationResult<()> {
        if self.0.len() < 8 {
            return Err(PasswordValidationError::TooShort);
        }

        if self.0.len() > 128 {
            return Err(PasswordValidationError::TooLong);
        }

        let re_numbers = Regex::new(r"\d+").unwrap();
        let re_capital = Regex::new(r"[A-Z!,.?@$%]+").unwrap();
        let re_special = Regex::new(r"[,.?@$%]+").unwrap();

        if !re_numbers.is_match(&self.0) {
            return Err(PasswordValidationError::NoNumbers);
        }

        if !re_capital.is_match(&self.0) {
            return Err(PasswordValidationError::NoCapitalLetters);
        }

        if !re_special.is_match(&self.0) {
            return Err(PasswordValidationError::NoSpecialCharacters);
        }

        Ok(())
    }
}
