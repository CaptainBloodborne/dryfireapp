//! Background-task scheduler.
//!
//! Currently hosts one job: [`license_reminders`]. The pattern: each
//! job is a `pub async fn run(state: AppState, interval: Duration)`
//! spawned from `init_app` as a detached tokio task. On shutdown the
//! main task dropping `AppState`'s pool will cause the next
//! DB-touching tick to error and the task to exit cleanly.

pub mod license_reminders;
