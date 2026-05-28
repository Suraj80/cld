use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

#[derive(Clone)]
pub struct IdentityKey {
    private: StaticSecret,
    public: PublicKey,
}

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Initiator,
    Listener,
}

#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SessionKeys {
    pub send_key: [u8; 32],
    pub recv_key: [u8; 32],
}

pub fn identity_key_path() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?
        .join("cld");

    fs::create_dir_all(&data_dir)?;

    Ok(data_dir.join("identity.key"))
}

pub fn load_or_create_identity() -> Result<IdentityKey> {
    let path = identity_key_path()?;

    if path.exists() {
        let encoded = fs::read_to_string(path)?;
        let bytes = general_purpose::STANDARD.decode(encoded.trim())?;

        let private_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid identity key length"))?;

        let private = StaticSecret::from(private_bytes);
        let public = PublicKey::from(&private);

        return Ok(IdentityKey { private, public });
    }

    let private = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&private);

    let encoded = general_purpose::STANDARD.encode(private.to_bytes());
    fs::write(path, encoded)?;

    Ok(IdentityKey { private, public })
}

impl IdentityKey {
    pub fn public_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.public.as_bytes())
    }

    pub fn derive_session_keys(
        &self,
        peer_public_key_base64: &str,
        my_salt: &[u8; 32],
        peer_salt: &[u8; 32],
        role: Role,
    ) -> Result<SessionKeys> {
        let peer_public_bytes = general_purpose::STANDARD.decode(peer_public_key_base64)?;

        let peer_public_array: [u8; 32] = peer_public_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid peer public key length"))?;

        let peer_public = PublicKey::from(peer_public_array);
        let shared_secret = self.private.diffie_hellman(&peer_public);

        let combined_salt = xor_salts(my_salt, peer_salt);

        let hk = Hkdf::<Sha256>::new(Some(&combined_salt), shared_secret.as_bytes());

        let mut a_to_b = [0u8; 32];
        let mut b_to_a = [0u8; 32];

        hk.expand(b"cld-v1-A-to-B", &mut a_to_b)
            .map_err(|_| anyhow::anyhow!("Failed to expand A-to-B session key"))?;

        hk.expand(b"cld-v1-B-to-A", &mut b_to_a)
            .map_err(|_| anyhow::anyhow!("Failed to expand B-to-A session key"))?;

        let keys = match role {
            Role::Initiator => SessionKeys {
                send_key: a_to_b,
                recv_key: b_to_a,
            },
            Role::Listener => SessionKeys {
                send_key: b_to_a,
                recv_key: a_to_b,
            },
        };

        Ok(keys)
    }
}

pub fn generate_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn salt_base64(salt: &[u8; 32]) -> String {
    general_purpose::STANDARD.encode(salt)
}

pub fn parse_salt_base64(value: &str) -> Result<[u8; 32]> {
    let bytes = general_purpose::STANDARD.decode(value)?;

    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid salt length"))
}

fn xor_salts(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];

    for i in 0..32 {
        out[i] = a[i] ^ b[i];
    }

    out
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn build_nonce(salt: &[u8; 32], counter: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];

    nonce[0..4].copy_from_slice(&salt[0..4]);
    nonce[4..12].copy_from_slice(&counter.to_le_bytes());

    nonce
}

pub fn encrypt_payload(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(nonce);

    cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| anyhow::anyhow!("Failed to encrypt payload"))
}

pub fn decrypt_payload(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(nonce);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Failed to decrypt payload"))
}

pub fn public_key_fingerprint_base64(public_key_base64: &str) -> Result<String> {
    let bytes = general_purpose::STANDARD.decode(public_key_base64)?;
    let hash = Sha256::digest(&bytes);
    Ok(bytes_to_hex(&hash))
}
