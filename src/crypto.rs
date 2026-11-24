use anyhow::Context;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

const NONCE_LENGTH: usize = 12;

#[derive(Clone)]
pub struct SecretBox {
    cipher: ChaCha20Poly1305,
}

impl SecretBox {
    pub fn from_hex(key: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(key.trim()).context("SECRET_KEY is not valid hex")?;
        anyhow::ensure!(
            bytes.len() == 32,
            "SECRET_KEY must be 32 bytes (64 hex characters)"
        );
        let cipher = ChaCha20Poly1305::new_from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("invalid SECRET_KEY"))?;
        Ok(Self { cipher })
    }

    pub fn generate_hex() -> String {
        hex::encode(rand::random::<[u8; 32]>())
    }

    pub fn seal(&self, plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
        let nonce_bytes = rand::random::<[u8; NONCE_LENGTH]>();
        let ciphertext = self
            .cipher
            .encrypt(&Nonce::from(nonce_bytes), plaintext)
            .map_err(|_| anyhow::anyhow!("encryption failed"))?;
        let mut sealed = nonce_bytes.to_vec();
        sealed.extend(ciphertext);
        Ok(sealed)
    }

    pub fn open(&self, sealed: &[u8]) -> anyhow::Result<Vec<u8>> {
        anyhow::ensure!(sealed.len() > NONCE_LENGTH, "sealed value is too short");
        let (nonce, ciphertext) = sealed.split_at(NONCE_LENGTH);
        let nonce: [u8; NONCE_LENGTH] = nonce.try_into()?;
        self.cipher
            .decrypt(&Nonce::from(nonce), ciphertext)
            .map_err(|_| anyhow::anyhow!("decryption failed, was SECRET_KEY changed?"))
    }
}

#[cfg(test)]
mod tests {
    use super::SecretBox;

    #[test]
    fn sealed_values_round_trip_and_differ_each_time() {
        let secrets = SecretBox::from_hex(&SecretBox::generate_hex()).unwrap();
        let first = secrets.seal(b"ptla_example").unwrap();
        let second = secrets.seal(b"ptla_example").unwrap();
        assert_ne!(first, second);
        assert_eq!(secrets.open(&first).unwrap(), b"ptla_example");
    }

    #[test]
    fn a_different_key_cannot_open_a_value() {
        let sealed = SecretBox::from_hex(&SecretBox::generate_hex())
            .unwrap()
            .seal(b"secret")
            .unwrap();
        let other = SecretBox::from_hex(&SecretBox::generate_hex()).unwrap();
        assert!(other.open(&sealed).is_err());
    }

    #[test]
    fn rejects_short_keys() {
        assert!(SecretBox::from_hex("abcd").is_err());
    }
}
