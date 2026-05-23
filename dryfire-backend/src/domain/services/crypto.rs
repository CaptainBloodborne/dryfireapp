use async_trait::async_trait;

#[async_trait]
pub trait Hasher: Send + Sync {
    async fn hash(&self, content: String) -> anyhow::Result<String>;
    async fn validate(&self, content: String, hash: String) -> anyhow::Result<()>;
}

pub trait Signer: Send + Sync {
    /// HMAC-SHA256 of `content` with the server's secret key,
    /// returned as raw bytes.
    fn sign(&self, content: &str) -> Vec<u8>;

    /// Verify the bytes (`provided`) against a fresh signature of
    /// `content`. Constant-time comparison.
    fn verify(&self, content: &str, provided: &[u8]) -> bool;
}
