use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key as AesKey, Nonce 
};
use aes_gcm::aead::rand_core::RngCore;
use argon2::{
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use std::sync::Mutex;


// 32 bytes for AES-256
pub type Key = [u8; 32];

pub struct VaultManager {
    pub state: Mutex<VaultState>,
    pub last_used_id: Mutex<Option<String>>,
}

#[derive(Debug, Clone)]
pub enum VaultState {
    Locked,
    Unlocked(Key),
}

impl VaultManager {
    pub fn new() -> Self {
        VaultManager {
            state: Mutex::new(VaultState::Locked),
            last_used_id: Mutex::new(None),
        }
    }
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
    password_hash.to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}

pub fn derive_key_from_password(password: &str, salt_str: &str) -> Key {
     let mut output_key = [0u8; 32];
     // Use Argon2 to derive a key. We need a consistent salt.
     // In a real scenario, we'd store a dedicated salt for key derivation, distinct from the password hash salt.
     // For this implementation, we will assume 'salt_str' is provided (e.g., from config).
     // If not, we'd need to generate and store it.
     
     // Simplified: Just use the password's hash (if we have it) to verify, 
     // but to DERIVE the encryption key, we should rely on a stable salt linked to the user's vault.
     // Let's assume we pass the salt bytes directly or a string representation.
     
    let salt_bytes = salt_str.as_bytes(); // simplistic
    Argon2::default().hash_password_into(
        password.as_bytes(),
        salt_bytes, 
        &mut output_key
    ).expect("Key derivation failed");
    
    output_key
}


pub fn encrypt(data: &str, key: &Key) -> Result<(String, String), String> {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, data.as_bytes())
        .map_err(|e| e.to_string())?;
    
    Ok((
        hex::encode(ciphertext),
        hex::encode(nonce_bytes)
    ))
}

pub fn decrypt(ciphertext_hex: &str, nonce_hex: &str, key: &Key) -> Result<String, String> {
    let ciphertext = hex::decode(ciphertext_hex).map_err(|_| "Invalid hex".to_string())?;
    let nonce_bytes = hex::decode(nonce_hex).map_err(|_| "Invalid hex".to_string())?;
    
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Decryption failed".to_string())?;
            
    String::from_utf8(plaintext).map_err(|_| "Invalid UTF-8".to_string())
}
