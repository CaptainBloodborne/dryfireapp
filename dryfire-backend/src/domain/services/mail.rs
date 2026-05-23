//! Outbound email port.

use async_trait::async_trait;

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send_verification_email(
        &self,
        to: &str,
        verification_url: &str,
    ) -> anyhow::Result<()>;

    async fn send_password_reset_email(
        &self,
        to: &str,
        reset_url: &str,
    ) -> anyhow::Result<()>;
}
