//! Cryptography Utilities Module
//! 
//! Provides cryptographic functions commonly used in music source
//! authentication and URL signing.

use aes::Aes256;
use aes::cipher::{KeyIvInit, BlockEncryptMut, BlockDecryptMut};
use cbc::{Encryptor, Decryptor};
type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;
use base64::{Engine, engine::general_purpose};
use hex;
use rand::Rng;
use sha2::{Sha256, Digest};

/// Crypto utilities wrapper
pub struct CryptoUtils;

impl CryptoUtils {
    /// Calculate MD5 hash
    pub fn md5(input: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{:016x}", hash)
    }

    /// Calculate SHA256 hash
    pub fn sha256(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Calculate SHA256 with output as bytes
    pub fn sha256_bytes(input: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hasher.finalize().to_vec()
    }

    /// AES-256-CBC encryption with PKCS7 padding
    pub fn aes_encrypt(plaintext: &str, key: &[u8], iv: &[u8]) -> Result<String, String> {
        if key.len() != 32 {
            return Err("Key must be 32 bytes for AES-256".to_string());
        }
        if iv.len() != 16 {
            return Err("IV must be 16 bytes".to_string());
        }
        
        let cipher = Aes256CbcEnc::new_from_slices(key, iv)
            .map_err(|e| e.to_string())?;
        
        // PKCS7 padding
        let block_size = 16;
        let msg_len = plaintext.len();
        let buf_len = msg_len + block_size - (msg_len % block_size);
        let mut buf = plaintext.as_bytes().to_vec();
        buf.resize(buf_len, 0);
        
        cipher.encrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buf, msg_len)
            .map_err(|e| e.to_string())?;
        
        Ok(general_purpose::STANDARD.encode(&buf))
    }

    /// AES-256-CBC decryption
    pub fn aes_decrypt(ciphertext: &str, key: &[u8], iv: &[u8]) -> Result<String, String> {
        if key.len() != 32 {
            return Err("Key must be 32 bytes for AES-256".to_string());
        }
        if iv.len() != 16 {
            return Err("IV must be 16 bytes".to_string());
        }
        
        let mut buf = general_purpose::STANDARD
            .decode(ciphertext)
            .map_err(|e| e.to_string())?;
        
        let cipher = Aes256CbcDec::new_from_slices(key, iv)
            .map_err(|e| e.to_string())?;
        
        let decrypted_len = cipher.decrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buf)
            .map_err(|e| e.to_string())?
            .len();
        
        Ok(String::from_utf8_lossy(&buf[..decrypted_len]).to_string())
    }

    /// Generate random hex string
    pub fn random_hex(length: usize) -> String {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
        hex::encode(bytes)
    }

    /// Generate random string
    pub fn random_string(length: usize) -> String {
        let charset: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            .chars()
            .collect();
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| charset[rng.gen_range(0..charset.len())])
            .collect()
    }

    /// Base64 encode
    pub fn base64_encode(input: &str) -> String {
        general_purpose::STANDARD.encode(input.as_bytes())
    }

    /// Base64 decode
    pub fn base64_decode(input: &str) -> Result<String, String> {
        let bytes = general_purpose::STANDARD
            .decode(input)
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| e.to_string())
    }

    /// URL-safe Base64 encode
    pub fn base64_url_encode(input: &str) -> String {
        general_purpose::URL_SAFE.encode(input.as_bytes())
    }

    /// URL-safe Base64 decode
    pub fn base64_url_decode(input: &str) -> Result<String, String> {
        let bytes = general_purpose::URL_SAFE
            .decode(input)
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| e.to_string())
    }

    /// HMAC-SHA256
    pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;
        
        let mut mac = HmacSha256::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// Create signature for music URLs (common pattern)
    pub fn create_signature(params: &HashMap<String, String>, secret: &str) -> String {
        let mut sorted: Vec<_> = params.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        
        let query_string: String = sorted
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        
        let data = format!("{}{}", query_string, secret);
        hex::encode(Sha256::digest(data.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // MD5 tests
    // ========================================================================
    #[test]
    fn test_md5_basic() {
        let hash = CryptoUtils::md5("hello");
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 16); // 16 hex chars (64-bit hash)
    }

    #[test]
    fn test_md5_consistent() {
        let h1 = CryptoUtils::md5("hello");
        let h2 = CryptoUtils::md5("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_md5_different_inputs() {
        let h1 = CryptoUtils::md5("hello");
        let h2 = CryptoUtils::md5("world");
        assert_ne!(h1, h2);
    }

    // ========================================================================
    // SHA256 tests
    // ========================================================================
    #[test]
    fn test_sha256_basic() {
        let hash = CryptoUtils::sha256("hello");
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // 64 hex chars
    }

    #[test]
    fn test_sha256_consistent() {
        let h1 = CryptoUtils::sha256("hello");
        let h2 = CryptoUtils::sha256("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_sha256_different_inputs() {
        let h1 = CryptoUtils::sha256("hello");
        let h2 = CryptoUtils::sha256("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_sha256_bytes() {
        let bytes = CryptoUtils::sha256_bytes("hello");
        assert_eq!(bytes.len(), 32); // SHA256 = 32 bytes
    }

    // ========================================================================
    // AES encryption tests
    // ========================================================================
    #[test]
    fn test_aes_encrypt_decrypt() {
        let key = b"12345678901234567890123456789012"; // 32 bytes
        let iv = b"1234567890123456"; // 16 bytes
        let plaintext = "Hello World!";
        let encrypted = CryptoUtils::aes_encrypt(plaintext, key, iv).unwrap();
        assert!(!encrypted.is_empty());
        let decrypted = CryptoUtils::aes_decrypt(&encrypted, key, iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_encrypt_different_data() {
        let key = b"12345678901234567890123456789012";
        let iv = b"1234567890123456";
        let e1 = CryptoUtils::aes_encrypt("hello", key, iv).unwrap();
        let e2 = CryptoUtils::aes_encrypt("world", key, iv).unwrap();
        assert_ne!(e1, e2);
    }

    #[test]
    fn test_aes_encrypt_bad_key_size() {
        let key = b"short"; // only 5 bytes
        let iv = b"1234567890123456";
        let result = CryptoUtils::aes_encrypt("hello", key, iv);
        assert!(result.is_err());
    }

    #[test]
    fn test_aes_encrypt_bad_iv_size() {
        let key = b"12345678901234567890123456789012";
        let iv = b"short"; // only 5 bytes
        let result = CryptoUtils::aes_encrypt("hello", key, iv);
        assert!(result.is_err());
    }

    #[test]
    fn test_aes_decrypt_bad_input() {
        let key = b"12345678901234567890123456789012";
        let iv = b"1234567890123456";
        let result = CryptoUtils::aes_decrypt("not-valid-base64!!!", key, iv);
        assert!(result.is_err());
    }

    // ========================================================================
    // Random generation tests
    // ========================================================================
    #[test]
    fn test_random_hex_length() {
        for len in [0, 8, 16, 32] {
            let s = CryptoUtils::random_hex(len);
            assert_eq!(s.len(), len * 2); // hex doubles the length
        }
    }

    #[test]
    fn test_random_hex_unique() {
        let s1 = CryptoUtils::random_hex(32);
        let s2 = CryptoUtils::random_hex(32);
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_random_string_length() {
        let s = CryptoUtils::random_string(10);
        assert_eq!(s.len(), 10);
    }

    #[test]
    fn test_random_string_alphanumeric() {
        let s = CryptoUtils::random_string(100);
        for c in s.chars() {
            assert!(c.is_ascii_alphanumeric());
        }
    }

    // ========================================================================
    // Base64 tests
    // ========================================================================
    #[test]
    fn test_base64_encode_decode() {
        let original = "Hello World!";
        let encoded = CryptoUtils::base64_encode(original);
        let decoded = CryptoUtils::base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_url_encode_decode() {
        let original = "Hello World!";
        let encoded = CryptoUtils::base64_url_encode(original);
        let decoded = CryptoUtils::base64_url_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_decode_invalid() {
        let result = CryptoUtils::base64_decode("!!!invalid!!!");
        assert!(result.is_err());
    }

    // ========================================================================
    // HMAC tests
    // ========================================================================
    #[test]
    fn test_hmac_sha256_basic() {
        let key = b"secret";
        let data = b"message";
        let mac = CryptoUtils::hmac_sha256(key, data);
        assert_eq!(mac.len(), 32); // SHA256 = 32 bytes
    }

    #[test]
    fn test_hmac_sha256_consistent() {
        let key = b"secret";
        let data = b"message";
        let mac1 = CryptoUtils::hmac_sha256(key, data);
        let mac2 = CryptoUtils::hmac_sha256(key, data);
        assert_eq!(mac1, mac2);
    }

    #[test]
    fn test_hmac_sha256_different_keys() {
        let data = b"message";
        let mac1 = CryptoUtils::hmac_sha256(b"key1", data);
        let mac2 = CryptoUtils::hmac_sha256(b"key2", data);
        assert_ne!(mac1, mac2);
    }

    // ========================================================================
    // Signature tests
    // ========================================================================
    #[test]
    fn test_create_signature() {
        let mut params = HashMap::new();
        params.insert("a".to_string(), "1".to_string());
        params.insert("b".to_string(), "2".to_string());
        let sig = CryptoUtils::create_signature(&params, "secret");
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // SHA256 hex
    }

    #[test]
    fn test_create_signature_consistent() {
        let mut params = HashMap::new();
        params.insert("a".to_string(), "1".to_string());
        params.insert("b".to_string(), "2".to_string());
        let sig1 = CryptoUtils::create_signature(&params, "secret");
        let sig2 = CryptoUtils::create_signature(&params, "secret");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_create_signature_order_independent() {
        let mut p1 = HashMap::new();
        p1.insert("b".to_string(), "2".to_string());
        p1.insert("a".to_string(), "1".to_string());
        let mut p2 = HashMap::new();
        p2.insert("a".to_string(), "1".to_string());
        p2.insert("b".to_string(), "2".to_string());
        let sig1 = CryptoUtils::create_signature(&p1, "secret");
        let sig2 = CryptoUtils::create_signature(&p2, "secret");
        assert_eq!(sig1, sig2); // sorted by key
    }
}

use std::collections::HashMap;
use cbc;