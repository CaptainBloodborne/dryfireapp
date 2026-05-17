use std::sync::Arc;

use  anyhow::anyhow;
use async_trait::async_trait;

use crate::{
    domain::services::{
        crypto::Hasher,
        identity::{Token, TokenHandler},
    },
    utils::{
        b64::{b64_decode, b64_encode},
        time::{now_utc_plus_sec_str, parse_utc, utc_now},
    },
};

pub struct TokenGenerator;

#[async_trait]
impl TokenHandler for TokenGenerator {
    async fn generate_token<T: Hasher>(hasher: Arc<T>, ident: &str) -> anyhow::Result<Token> {
        let exp = now_utc_plus_sec_str(1800);

        let sign = hasher
            .hash(format!("{}.{}", b64_encode(ident), b64_encode(&exp)))
            .await?;

        let sign_b64u = b64_encode(&sign);

        anyhow::Ok(Token {
            ident: ident.to_string(),
            exp: exp,
            sign_b64u: sign_b64u,
        })
    }
    
    async fn validate_token<T: Hasher>(
        hasher: Arc<T>,
        original_token: &Token,
    ) -> anyhow::Result<()> {
        let ident_and_exp_b64u = format!(
            "{}.{}",
            b64_encode(&original_token.ident),
            b64_encode(&original_token.exp)
        );

        let sign =
            b64_decode(&original_token.sign_b64u)?;

        hasher
            .validate(ident_and_exp_b64u, sign)
            .await?;

        let origin_exp = parse_utc(&original_token.exp)?;

        let now = utc_now();

        println!("->> token validation - now: {now:?} - origin_exp: {origin_exp:?}");

        if origin_exp < now {
            return Err(anyhow!(TokenError::TokenExpired));
        }

        anyhow::Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("Can not match tokens")]
    TokenNotMatch,

    #[error("error")]
    TokenExpired,

    #[error("error")]
    TokenExpNotIso,

    #[error("error")]
    TokenCannotDecode,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::infra::hash::argon::ArgonHasher;

    use super::*;
    use anyhow::Result;
    use tokio::time::sleep;

    #[test]
    fn test_token() -> Result<()> {
        let fx_token = Token {
            ident: "fx-user".to_string(),
            exp: "2026-05-17T15:30:00Z".to_string(),
            sign_b64u: "sign".to_string(),
        };

        println!("-->> {}", fx_token);

        Ok(())
    }

    #[test]
    fn test_token_from_str_ok() -> anyhow::Result<()> {
        let fx_token_str = "ZngtdXNlcg.MjAyNi0wNS0xN1QxNTozMDowMFo.sign";

        let fx_token = Token {
            ident: "fx-user".to_string(),
            exp: "2026-05-17T15:30:00Z".to_string(),
            sign_b64u: "sign".to_string(),
        };

        let token = fx_token_str.parse::<Token>()?;

        assert_eq!(format!("{:?}", token), format!("{:?}", fx_token));

        Ok(())
    }

    #[tokio::test]
    async fn test_token_validation() -> anyhow::Result<()> {
        let hasher = Arc::new(ArgonHasher);

        let ident = "fx_user";

        let fx_token = TokenGenerator::generate_token(hasher.clone(), ident).await?;

        sleep(Duration::from_millis(10)).await;

        TokenGenerator::validate_token(hasher, &fx_token).await?;

        Ok(())
    }
}
