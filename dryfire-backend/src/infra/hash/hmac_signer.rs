//! HMAC-SHA256 signer for session tokens.
//!
//! Unlike `ArgonHasher`, this is fast (single-digit microseconds) and
//! deterministic (no salt). That makes it correct for per-request
//! token validation but **wrong** for password storage — keep the two
//! traits separate.

use hmac::{Hmac, Mac, KeyInit};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::domain::services::crypto::Signer;

type HmacSha256 = Hmac<Sha256>;

pub struct HmacSigner {
    key: Vec<u8>,
}

impl HmacSigner {
    pub fn new(key: &[u8]) -> Self {

        Self { key: key.to_vec() }
    }
}

impl Signer for HmacSigner {
    fn sign(&self, content: &str) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(content.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    fn verify(&self, content: &str, provided: &[u8]) -> bool {
        let expected = self.sign(content);

        expected.ct_eq(provided).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_roundtrip() {
        let s = HmacSigner::new(b"some-key-of-reasonable-length-1234");
        let sig = s.sign("hello");
        assert!(s.verify("hello", &sig));
        assert!(!s.verify("hellp", &sig));
        assert!(!s.verify("hello", &[]));
    }

    #[test]
    fn different_keys_produce_different_sigs() {
        let a = HmacSigner::new(b"key-a-key-a-key-a-key-a-key-a-key-a");
        let b = HmacSigner::new(b"key-b-key-b-key-b-key-b-key-b-key-b");
        assert_ne!(a.sign("hello"), b.sign("hello"));
    }
}
