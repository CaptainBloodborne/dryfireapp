
use anyhow::Ok;
use dotenvy::dotenv;

use dryfire_backend::infra;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    infra::init_app().await?;

    Ok(())
}
