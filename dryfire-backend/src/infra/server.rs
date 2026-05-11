use async_trait::async_trait;

use crate::application::app_state::AppState;


#[async_trait]
pub trait Server: Send + Sync {
    async fn start_server(&self, state: AppState) -> anyhow::Result<()>;
    fn shutdown(&self) -> anyhow::Result<()>;
}