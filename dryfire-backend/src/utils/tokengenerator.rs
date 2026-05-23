//! Token generation / validation using a fast keyed MAC.
//!
//! Token wire format: `b64u(ident).b64u(exp).b64u(HMAC(ident_b64u.exp_b64u))`
//!
//! The previous implementation took a `Hasher` (Argon2) generic parameter —
//! that worked but used a slow password-hashing primitive on the hot path
//! and forced every caller to know about Argon2. This version:
//!
//! 1. Uses [`Signer`] (HMAC-SHA256) — micro-second-fast, constant-time verify.
//! 2. Stores the signer in the generator so callers see only a clean trait.
//! 3. Verifies signature **before** parsing expiry, so a malformed-token
//!    error can't be distinguished from an expired-token error by timing
//!    (mild defence against probing).

use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;

use crate::{
    domain::services::{
        crypto::Signer,
        identity::{Token, TokenHandler},
    },
    utils::{
        b64::{b64_decode_bytes, b64_encode, b64_encode_bytes},
        time::{now_utc_plus_sec_str, parse_utc, utc_now},
    },
};

/// Default access-token TTL in seconds (30 min).
pub const DEFAULT_TOKEN_TTL_SECS: i64 = 1800;

pub struct TokenGenerator {
    signer: Arc<dyn Signer>,
    ttl_secs: i64,
}

impl TokenGenerator {
    pub fn new(signer: Arc<dyn Signer>) -> Self {
        Self { signer, ttl_secs: DEFAULT_TOKEN_TTL_SECS }
    }

    pub fn with_ttl(signer: Arc<dyn Signer>, ttl_secs: i64) -> Self {
        Self { signer, ttl_secs }
    }
}

#[async_trait]
impl TokenHandler for TokenGenerator {
    async fn generate_token(&self, ident: &str) -> anyhow::Result<Token> {
        let exp = now_utc_plus_sec_str(self.ttl_secs);
        let signed_payload =
            format!("{}.{}", b64_encode(ident), b64_encode(&exp));
        let sig_bytes = self.signer.sign(&signed_payload);
        let sign_b64u = b64_encode_bytes(&sig_bytes);

        Ok(Token { ident: ident.to_string(), exp, sign_b64u })
    }

    async fn validate_token(&self, token: &Token) -> anyhow::Result<()> {
        let signed_payload = format!(
            "{}.{}",
            b64_encode(&token.ident),
            b64_encode(&token.exp),
        );
        let provided = b64_decode_bytes(&token.sign_b64u)
            .map_err(|_| anyhow!(TokenError::TokenCannotDecode))?;

        if !self.signer.verify(&signed_payload, &provided) {
            return Err(anyhow!(TokenError::TokenNotMatch));
        }

        let exp = parse_utc(&token.exp)
            .map_err(|_| anyhow!(TokenError::TokenExpNotIso))?;
        if exp < utc_now() {
            return Err(anyhow!(TokenError::TokenExpired));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("token signature does not match")]
    TokenNotMatch,
    #[error("token expired")]
    TokenExpired,
    #[error("token expiry is not RFC3339")]
    TokenExpNotIso,
    #[error("token cannot be decoded")]
    TokenCannotDecode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::hash::hmac_signer::HmacSigner;
    use std::time::Duration;
    use tokio::time::sleep;

    fn signer() -> Arc<dyn Signer> {
        Arc::new(HmacSigner::new(b"a-test-key-that-is-at-least-32-bytes-long!"))
    }

    #[tokio::test]
    async fn happy_path() {
        let tokengen = TokenGenerator::new(signer());
        let t = tokengen.generate_token("user-1").await.unwrap();
        tokengen.validate_token(&t).await.unwrap();
    }

    #[tokio::test]
    async fn tampered_signature_rejected() {
        let tokengen = TokenGenerator::new(signer());
        let mut t = tokengen.generate_token("user-1").await.unwrap();
        t.sign_b64u.push('A');
        let err = tokengen.validate_token(&t).await.unwrap_err();
        assert!(err.to_string().contains("does not match")
                || err.to_string().contains("cannot be decoded"));
    }

    #[tokio::test]
    async fn expired_token_rejected() {
        let tokengen = TokenGenerator::with_ttl(signer(), 0);
        let t = tokengen.generate_token("user-1").await.unwrap();
        sleep(Duration::from_millis(100)).await;
        let err = tokengen.validate_token(&t).await.unwrap_err();
        assert!(err.to_string().contains("expired"));
    }
    #[tokio::test]
    async fn parses_from_wire_format() {
        let tokengen = TokenGenerator::new(signer());
        let t = tokengen.generate_token("user-2").await.unwrap();
        let wire = t.to_string();
        let parsed: Token = wire.parse().unwrap();
        assert_eq!(parsed, t);
    }
}
