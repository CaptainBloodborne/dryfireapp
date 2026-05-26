//! No-op mailer that just logs the message. Lets the server boot and
//! integration-test the flow without an SMTP server. Swap for the
//! `lettre`-based SMTP impl in production.

use async_trait::async_trait;

use crate::domain::services::mail::Mailer;

pub struct LoggingMailer;

#[async_trait]
impl Mailer for LoggingMailer {
    async fn send_verification_email(
        &self,
        to: &str,
        verification_url: &str,
    ) -> anyhow::Result<()> {
        tracing::info!(
            target = to,
            url = verification_url,
            "- would send verification email"
        );
        Ok(())
    }

    async fn send_password_reset_email(
        &self,
        to: &str,
        reset_url: &str,
    ) -> anyhow::Result<()> {
        tracing::info!(
            target = to,
            url = reset_url,
            "- would send password reset email"
        );
        Ok(())
    }

    async fn send_notification(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        tracing::info!(target: "mail", %to, %subject, %body, "notification email");
        Ok(())
    }
}
