use anyhow::anyhow;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use async_trait::async_trait;

use crate::domain::services::crypto::Hasher;

pub struct ArgonHasher;

#[async_trait]
impl Hasher for ArgonHasher {
    async fn hash(&self, content: String) -> anyhow::Result<String> {
        let result = tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);

            let hasher = Argon2::default();

            let password_hash = hasher
                .hash_password(content.as_bytes(), &salt)
                .map_err(|e| anyhow!(e))?
                .to_string();

            Ok(password_hash)
        })
        .await?;

        result
    }

    async fn validate(&self, content: String, hash: String) -> anyhow::Result<()> {
        let result = tokio::task::spawn_blocking(move || {
            let parsed_hash = PasswordHash::new(&hash).map_err(|e| anyhow!(e))?;

            let hasher = Argon2::default();

            hasher
                .verify_password(content.as_bytes(), &parsed_hash)
                .map_err(|e| anyhow!(e))?;

            Ok(())
        })
        .await?;

        result
    }
}
