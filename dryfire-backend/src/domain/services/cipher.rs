//! Field-level encryption port.
//!
//! Strategy:
//! - **AES-256-GCM** (authenticated encryption — detects tampering).
//! - Per-record random 12-byte nonce, stored prepended to the ciphertext.
//! - DB column type: `BYTEA` (or `TEXT` of base64 if we ever need to
//!   port to a backend without binary support).
//!
//! The key is a 32-byte secret loaded from config. The wire format
//! also includes a 1-byte `key_id` so we can rotate keys later without
//! re-encrypting everything in a single migration:
//!
//! `[key_id (1B)] [nonce (12B)] [ciphertext + tag]`

use async_trait::async_trait;

#[async_trait]
pub trait FieldCipher: Send + Sync {
    /// Encrypt a plaintext into the wire format above.
    fn encrypt(&self, plaintext: &[u8]) -> anyhow::Result<Vec<u8>>;

    /// Decrypt; verifies the GCM tag.
    fn decrypt(&self, blob: &[u8]) -> anyhow::Result<Vec<u8>>;

    /// Convenience for the common case (UTF-8 strings).
    fn encrypt_str(&self, s: &str) -> anyhow::Result<Vec<u8>> {
        self.encrypt(s.as_bytes())
    }
    fn decrypt_str(&self, blob: &[u8]) -> anyhow::Result<String> {
        let bytes = self.decrypt(blob)?;
        Ok(String::from_utf8(bytes)?)
    }
}
