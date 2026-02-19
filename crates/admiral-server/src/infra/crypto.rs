//! AES-256-GCM encrypt/decrypt for OAuth token storage.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
};
use rand::RngCore;

const NONCE_LEN: usize = 12;

/// Encrypt plaintext with key (32 bytes). Returns (ciphertext, nonce).
pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>), aes_gcm::Error> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| aes_gcm::Error)?;
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher.encrypt((&nonce).into(), plaintext)?;
    Ok((ciphertext, nonce.to_vec()))
}

/// Decrypt (ciphertext, nonce) with key.
pub fn decrypt(ciphertext: &[u8], nonce: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, aes_gcm::Error> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| aes_gcm::Error)?;
    let nonce_arr = aes_gcm::Nonce::from_slice(nonce);
    cipher.decrypt(nonce_arr, ciphertext)
}

/// Decode 32-byte key from hex string (64 chars).
pub fn key_from_hex(hex_str: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hex_str).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err("key must be 32 bytes (64 hex chars)".to_string());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}
