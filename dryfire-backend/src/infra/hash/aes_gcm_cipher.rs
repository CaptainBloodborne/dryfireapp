//! AES-256-GCM `FieldCipher` implementation.

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use anyhow::{Context, anyhow};

use crate::domain::services::cipher::FieldCipher;

const KEY_ID: u8 = 1;        // bump on rotation; keep old keys around for decrypt
const NONCE_LEN: usize = 12; // GCM standard

pub struct AesGcmCipher {
    cipher: Aes256Gcm,
}

impl AesGcmCipher {
    /// Build from a 32-byte key. Anything else is a config bug — we
    /// reject loudly rather than silently truncate.
    pub fn from_key_bytes(key: &[u8]) -> anyhow::Result<Self> {
        if key.len() != 32 {
            return Err(anyhow!(
                "FIELD_ENCRYPTION_KEY must be exactly 32 bytes (got {})",
                key.len()
            ));
        }
        let key = Key::<Aes256Gcm>::from_slice(key);
        Ok(Self { cipher: Aes256Gcm::new(key) })
    }
}

impl FieldCipher for AesGcmCipher {
    fn encrypt(&self, plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow!("aes-gcm encrypt failed: {e}"))?;

        // Pack as [key_id (1B)] [nonce (12B)] [ciphertext]
        let mut blob = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
        blob.push(KEY_ID);
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);
        Ok(blob)
    }

    fn decrypt(&self, blob: &[u8]) -> anyhow::Result<Vec<u8>> {
        if blob.len() < 1 + NONCE_LEN {
            return Err(anyhow!("ciphertext blob is too short"));
        }
        let key_id = blob[0];
        if key_id != KEY_ID {
            return Err(anyhow!("unknown encryption key id {key_id}"));
        }
        let nonce = Nonce::from_slice(&blob[1..1 + NONCE_LEN]);
        let ct = &blob[1 + NONCE_LEN..];

        self.cipher
            .decrypt(nonce, ct)
            .map_err(|e| anyhow!("aes-gcm decrypt failed (tag mismatch?): {e}"))
            .context("authenticated decryption failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> AesGcmCipher {
        AesGcmCipher::from_key_bytes(&[0xA5; 32]).unwrap()
    }

    #[test]
    fn roundtrip() {
        let c = cipher();
        let pt = b"AB1234567";
        let blob = c.encrypt(pt).unwrap();
        assert_ne!(&blob[..], &pt[..]);
        // Header (1) + nonce (12) + ciphertext (9) + GCM tag (16)
        assert_eq!(blob.len(), 1 + 12 + 9 + 16);
        assert_eq!(c.decrypt(&blob).unwrap(), pt);
    }

    #[test]
    fn each_encryption_has_unique_nonce() {
        let c = cipher();
        let a = c.encrypt(b"hello").unwrap();
        let b = c.encrypt(b"hello").unwrap();
        assert_ne!(a, b, "nonce must be random — equal ciphertexts leak repetition");
    }

    #[test]
    fn tampered_blob_rejected() {
        let c = cipher();
        let mut blob = c.encrypt(b"hello").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        assert!(c.decrypt(&blob).is_err());
    }

    #[test]
    fn wrong_key_rejected() {
        let c1 = cipher();
        let c2 = AesGcmCipher::from_key_bytes(&[0x42; 32]).unwrap();
        let blob = c1.encrypt(b"hello").unwrap();
        assert!(c2.decrypt(&blob).is_err());
    }

    #[test]
    fn rejects_wrong_key_length() {
        assert!(AesGcmCipher::from_key_bytes(&[0; 16]).is_err());
        assert!(AesGcmCipher::from_key_bytes(&[0; 64]).is_err());
    }
}
