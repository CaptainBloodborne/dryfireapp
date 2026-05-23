//! Identity-related value objects and traits.
//!
//! - [`Password`], [`Email`], [`Login`] — validated newtypes.
//! - [`Token`] — opaque session token (`ident.exp.sig` in base64url).
//! - [`TokenHandler`] — sign / verify tokens. Implemented in `infra`.
//! - [`Credentials`] — what the repository persists (password hash etc.).

use std::{fmt::Display, str::FromStr};

use chrono::{DateTime, Utc};
use regex::Regex;
use secrecy::{ExposeSecret, SecretString};
use std::sync::OnceLock;

use crate::{
    domain::errors::ValidationError,
    utils::b64::{b64_decode, b64_encode},
    utils::time::utc_now,
};


// Password

/// A password that has passed the strength policy. Wrap a
/// [`SecretString`] so it can't be accidentally logged or compared with
/// `==` (the inner string isn't `PartialEq`).
pub struct Password(SecretString);

impl Password {
    pub fn new(raw: SecretString) -> Result<Self, ValidationError> {
        Self::validate(raw.expose_secret())?;
        Ok(Self(raw))
    }

    /// For tests / fixtures.
    pub fn parse(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw: String = raw.into();
        Self::validate(&raw)?;
        Ok(Self(SecretString::from(raw)))
    }

    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }

    fn validate(s: &str) -> Result<(), ValidationError> {
        let len = s.len();
        if len < 8 { return Err(ValidationError::PasswordTooShort); }
        if len > 128 { return Err(ValidationError::PasswordTooLong); }

        // Compile each regex once.
        static RE_DIGIT: OnceLock<Regex> = OnceLock::new();
        static RE_UPPER: OnceLock<Regex> = OnceLock::new();
        static RE_LOWER: OnceLock<Regex> = OnceLock::new();
        static RE_SPECIAL: OnceLock<Regex> = OnceLock::new();

        let re_digit = RE_DIGIT.get_or_init(|| Regex::new(r"\d").unwrap());
        let re_upper = RE_UPPER.get_or_init(|| Regex::new(r"[A-Z]").unwrap());
        let re_lower = RE_LOWER.get_or_init(|| Regex::new(r"[a-z]").unwrap());
        let re_special = RE_SPECIAL.get_or_init(|| {
            Regex::new(r"[!@#$%^&*(),.?:;{}|<>\-_+=/\\\[\]'~`]").unwrap()
        });

        if !re_digit.is_match(s)   { return Err(ValidationError::PasswordNoDigit); }
        if !re_upper.is_match(s)   { return Err(ValidationError::PasswordNoUppercase); }
        if !re_lower.is_match(s)   { return Err(ValidationError::PasswordNoLowercase); }
        if !re_special.is_match(s) { return Err(ValidationError::PasswordNoSpecial); }
        Ok(())
    }
}

impl std::fmt::Debug for Password {
    // Never reveal the secret in {:?}.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Password").field("inner", &"***").finish()
    }
}

// Email
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn parse(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw: String = raw.into().trim().to_owned();

        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| {
            Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$")
                .unwrap()
        });
        if !re.is_match(&raw) || raw.len() > 254 {
            return Err(ValidationError::Email);
        }
        Ok(Self(raw.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str { &self.0 }
    pub fn into_string(self) -> String { self.0 }
}


// Login
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login(String);

impl Login {
    pub fn parse(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw: String = raw.into().trim().to_owned();
        if raw.len() < 3 || raw.len() > 64
            || !raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(ValidationError::Login);
        }
        Ok(Self(raw))
    }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn into_string(self) -> String { self.0 }
}

// Token
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub ident: String,        // session UUID (NOT user id, NOT email)
    pub exp: String,          // RFC 3339
    pub sign_b64u: String,    // base64url(HMAC-SHA256(...))
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            b64_encode(&self.ident),
            b64_encode(&self.exp),
            &self.sign_b64u,
        )
    }
}

impl FromStr for Token {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow::anyhow!("token must have three segments"));
        }
        Ok(Self {
            ident: b64_decode(parts[0])?,
            exp: b64_decode(parts[1])?,
            sign_b64u: parts[2].to_string(),
        })
    }
}


// TokenHandler

#[async_trait::async_trait]
pub trait TokenHandler: Send + Sync {
    /// Build a token for `ident` (typically a session UUID).
    async fn generate_token(&self, ident: &str) -> anyhow::Result<Token>;

    /// Verify signature + expiry. Returns Ok(()) iff the token is valid.
    async fn validate_token(&self, token: &Token) -> anyhow::Result<()>;
}

//Credentials

/// What the [`UserRepository`](crate::domain::repositories::user::UserRepository)
/// stores alongside the user — the argon2 PHC hash and audit fields.
#[derive(Debug, Clone)]
pub struct Credentials {
    password_hash: String,
    last_visit: DateTime<Utc>,
}

impl Credentials {
    pub fn new(password_hash: String) -> Self {
        Self { password_hash, last_visit: utc_now() }
    }

    pub fn password_hash(&self) -> &str { &self.password_hash }
    pub fn last_visit(&self) -> DateTime<Utc> { self.last_visit }
}


// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_policy() {
        assert!(Password::parse("Aa1!aaaa").is_ok());
        assert!(matches!(Password::parse("short"),
            Err(ValidationError::PasswordTooShort)));
        assert!(matches!(Password::parse("nouppercase1!"),
            Err(ValidationError::PasswordNoUppercase)));
        assert!(matches!(Password::parse("NOLOWERCASE1!"),
            Err(ValidationError::PasswordNoLowercase)));
        assert!(matches!(Password::parse("NoSpecials1"),
            Err(ValidationError::PasswordNoSpecial)));
        assert!(matches!(Password::parse("NoDigits!!"),
            Err(ValidationError::PasswordNoDigit)));
    }

    #[test]
    fn email_normalizes_and_validates() {
        let e = Email::parse("  Foo@Bar.COM ").unwrap();
        assert_eq!(e.as_str(), "foo@bar.com");
        assert!(Email::parse("not-an-email").is_err());
        assert!(Email::parse("a@b").is_err());
    }

    #[test]
    fn login_policy() {
        assert!(Login::parse("john_doe1").is_ok());
        assert!(Login::parse("ab").is_err());
        assert!(Login::parse("has space").is_err());
        assert!(Login::parse("has-dash").is_err());
    }

    #[test]
    fn token_roundtrip() {
        let t = Token {
            ident: "abc".into(),
            exp: "2026-05-17T15:30:00+00:00".into(),
            sign_b64u: "sig".into(),
        };
        let s = t.to_string();
        let parsed: Token = s.parse().unwrap();
        assert_eq!(parsed, t);
    }
}
