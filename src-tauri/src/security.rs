use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key as AesKey, Nonce,
};
use aes_gcm::aead::rand_core::RngCore;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::sync::Mutex;

const AES_KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

pub type Key = [u8; AES_KEY_SIZE];

pub struct VaultManager {
    pub state: Mutex<VaultState>,
    pub last_used_id: Mutex<Option<String>>,
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum VaultState {
    Locked,
    Unlocked(Key),
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(VaultState::Locked),
            last_used_id: Mutex::new(None),
        }
    }
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Password hashing failed")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Derives an AES-256 encryption key from a password using Argon2.
pub fn derive_key_from_password(password: &str, salt: &str) -> Key {
    let mut key = [0u8; AES_KEY_SIZE];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt.as_bytes(), &mut key)
        .expect("Key derivation failed");
    key
}

/// Encrypts data using AES-256-GCM. Returns (ciphertext_hex, nonce_hex).
pub fn encrypt(data: &str, key: &Key) -> Result<(String, String), String> {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key));
    
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|e| e.to_string())?;

    Ok((hex::encode(ciphertext), hex::encode(nonce_bytes)))
}

/// Decrypts AES-256-GCM encrypted data.
pub fn decrypt(ciphertext_hex: &str, nonce_hex: &str, key: &Key) -> Result<String, String> {
    let ciphertext = hex::decode(ciphertext_hex).map_err(|_| "Invalid ciphertext hex".to_string())?;
    let nonce_bytes = hex::decode(nonce_hex).map_err(|_| "Invalid nonce hex".to_string())?;

    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Decryption failed".to_string())?;

    String::from_utf8(plaintext).map_err(|_| "Invalid UTF-8".to_string())
}
