use async_trait::async_trait;

// type CryptoResult<T, E=CryptoError> = Result<T, E>;

#[async_trait]
pub trait Hasher: Send + Sync {
    async fn hash(&self, content: String) -> anyhow::Result<String>;
    async fn validate(&self, content: String, hash: String) -> anyhow::Result<()>;
}

// #[derive(Debug, thiserror::Error)]
// pub enum CryptoError{
//     #[error("Can not match encrypted data: {0}")]
//     ValidationFailed(String),

//     #[error("Can not encrypt data: {0}")]
//     HashingFailed(String),

// }