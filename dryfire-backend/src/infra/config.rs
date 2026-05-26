use secrecy::SecretString;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub controller: String,            // socket addr, e.g. "0.0.0.0:8000"

    /// Secret used by [`HmacSigner`](crate::infra::hash::hmac_signer::HmacSigner)
    /// to sign session tokens. Must be ≥ 32 bytes of random data.
    pub token_secret: String,

    /// 32-byte AES-256-GCM key for sensitive-field encryption.
    /// Provide as 64 hex chars (e.g. `openssl rand -hex 32`).
    pub field_encryption_key_hex: String,

    /// Public origin used in verification / password-reset email links,
    /// e.g. "https://dryfire.app".
    #[serde(default = "default_public_origin")]
    pub public_origin: String,

    /// Pool sizing.
    #[serde(default = "default_pool_max")]
    pub pool_max_connections: u32,

    /// Token TTLs in seconds.
    #[serde(default = "default_access_ttl")]
    pub access_token_ttl_secs: i64,
    #[serde(default = "default_session_ttl")]
    pub session_ttl_secs: i64,
    #[serde(default = "default_verify_ttl")]
    pub verify_token_ttl_secs: i64,
    #[serde(default = "default_reset_ttl")]
    pub reset_token_ttl_secs: i64,

    /// How often the license-reminder scheduler ticks. Defaults to
    /// 1 hour, which is plenty: reminders are date-based, not
    /// minute-based, and quieter ticks keep DB load down.
    #[serde(default = "default_scheduler_tick")]
    pub scheduler_tick_secs: u64,
}

fn default_public_origin() -> String { "http://localhost:8000".into() }
fn default_pool_max() -> u32 { 20 }
fn default_access_ttl() -> i64 { 1800 }                 // 30 min
fn default_session_ttl() -> i64 { 60 * 60 * 24 * 30 }   // 30 days
fn default_verify_ttl() -> i64 { 60 * 60 * 24 }         // 24 h
fn default_reset_ttl() -> i64 { 60 * 60 }               // 1 h
fn default_scheduler_tick() -> u64 { 60 * 60 }           // 1 h

impl Config {
    pub fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.controller.parse()?)
    }

    pub fn token_secret_secret(&self) -> SecretString {
        SecretString::from(self.token_secret.clone())
    }
}

pub fn init_config() -> anyhow::Result<Config> {
    // We don't print the Config wholesale — token_secret would leak.
    let cfg = envy::from_env::<Config>()
        .map_err(|e| anyhow::anyhow!("config error: {e}"))?;

    if cfg.token_secret.len() < 32 {
        anyhow::bail!("TOKEN_SECRET must be at least 32 bytes");
    }
    Ok(cfg)
}
