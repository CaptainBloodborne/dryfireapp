use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::infra::config::Config;

/// Build the Postgres connection pool.
pub async fn init_pool(config: &Config) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.pool_max_connections)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Some(Duration::from_secs(600)))
        .test_before_acquire(true)
        .connect(&config.database_url)
        .await?;
    Ok(pool)
}
