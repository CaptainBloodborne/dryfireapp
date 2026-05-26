//! License expiry-reminder scheduler.
//!
//! Polls every `interval` for licenses that are exactly 90/60/45/30/14
//! days from expiring AND haven't already received a reminder for that
//! threshold. For each match, looks up the owning user's email and
//! sends a notification via the [`Mailer`](crate::domain::services::mail::Mailer)
//! port — then records the send in `license_notifications` so we
//! never double-fire.


use std::time::Duration;

use chrono::Utc;
use tokio::time::sleep;

use crate::application::app_state::AppState;

/// Reminder thresholds, in days before expiry. Spec: 90, 60, 45, 30, 14.
pub const REMINDER_DAYS: &[i32] = &[90, 60, 45, 30, 14];

/// Entry point. Spawn this as `tokio::spawn(run(state, interval))`.
/// It never returns under normal operation.
#[tracing::instrument(skip(state), fields(interval_secs = interval.as_secs()))]
pub async fn run(state: AppState, interval: Duration) {
    tracing::info!("license-reminder scheduler starting");
    // First tick after a short delay so we don't fight startup.
    sleep(Duration::from_secs(5)).await;

    loop {
        if let Err(e) = tick(&state).await {
            // Don't crash the scheduler on transient errors; log and
            // wait for the next tick.
            tracing::error!(error = ?e, "scheduler tick failed");
        }
        sleep(interval).await;
    }
}

async fn tick(state: &AppState) -> anyhow::Result<()> {
    let today = Utc::now().date_naive();

    let due = state.license_repo
        .licenses_due_for_reminder(REMINDER_DAYS, today)
        .await?;
    if due.is_empty() {
        tracing::trace!("no reminders due");
        return Ok(());
    }
    tracing::info!(count = due.len(), "reminders to send");

    for item in due {
        // Look up the user to get their email.
        let user = match state.user_repo.find_by_id(item.user_id).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                tracing::warn!(license = %item.license_id, "user gone, skipping reminder");
                continue;
            }
            Err(e) => {
                tracing::error!(error = ?e, license = %item.license_id,
                    "failed to load user; will retry next tick");
                continue;
            }
        };

        let body = format!(
            "License {} (issued by …) expires on {} — that's {} days from now.",
            item.license_number, item.expires_at, item.days_before,
        );
        if let Err(e) = state.mailer.send_verification_email(user.email(), &body).await {
            tracing::error!(error = ?e, license = %item.license_id,
                "failed to send reminder; will retry next tick");
            continue;
        }

        if let Err(e) = state.license_notification_repo
            .mark_sent(item.license_id, item.days_before).await
        {
            tracing::error!(error = ?e, license = %item.license_id,
                "failed to record sent notification — risk of duplicate send next tick");
        }
    }
    Ok(())
}
