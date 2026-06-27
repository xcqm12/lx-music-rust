use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::cipher::generic_array::GenericArray;

/// MD5 哈希
pub fn md5(input: &str) -> String {
    let digest = md5::compute(input.as_bytes());
    format!("{:x}", digest)
}

/// MD5 哈希（大写）
pub fn md5_upper(input: &str) -> String {
    md5(input).to_uppercase()
}

/// AES ECB 加密
pub fn aes_ecb_encrypt(plaintext: &[u8], key: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::block_padding::Pkcs7;
    
    type Aes128Ecb = aes::cipher::Ecb::<Aes128, Pkcs7>;
    
    let cipher = Aes128Ecb::new_from_slices(key, &[]).unwrap();
    let mut buffer = plaintext.to_vec();
    let ciphertext = cipher.encrypt_padded_vec_mut::<Pkcs7>(&mut buffer);
    ciphertext.to_vec()
}

/// AES ECB 解密
pub fn aes_ecb_decrypt(ciphertext: &[u8], key: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::block_padding::Pkcs7;
    
    type Aes128Ecb = aes::cipher::Ecb::<Aes128, Pkcs7>;
    
    let cipher = Aes128Ecb::new_from_slices(key, &[]).unwrap();
    let mut buffer = ciphertext.to_vec();
    cipher.decrypt_padded_vec_mut::<Pkcs7>(&mut buffer).unwrap_or_default()
}

/// AES CBC 加密
pub fn aes_cbc_encrypt(plaintext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::block_padding::Pkcs7;
    
    type Aes128Cbc = aes::cipher::Cbc::<Aes128, Pkcs7>;
    
    let cipher = Aes128Cbc::new_from_slices(key, iv).unwrap();
    let mut buffer = plaintext.to_vec();
    cipher.encrypt_padded_vec_mut::<Pkcs7>(&mut buffer).to_vec()
}

/// AES CBC 解密
pub fn aes_cbc_decrypt(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::block_padding::Pkcs7;
    
    type Aes128Cbc = aes::cipher::Cbc::<Aes128, Pkcs7>;
    
    let cipher = Aes128Cbc::new_from_slices(key, iv).unwrap();
    let mut buffer = ciphertext.to_vec();
    cipher.decrypt_padded_vec_mut::<Pkcs7>(&mut buffer).unwrap_or_default()
}

/// RSA 加密（网易风格）
pub fn rsa_encrypt(input: &str, pubkey: &str, modulus: &str) -> String {
    use num_bigint::BigInt;
    use num_traits::{Zero, Num};
    
    let input_bytes = input.as_bytes();
    let input_rev: Vec<u8> = input_bytes.iter().rev().cloned().collect();
    let input_num = BigInt::from_bytes_be(num_bigint::Sign::Plus, &input_rev);
    
    let pubkey_num = BigInt::from_str_radix(pubkey, 16).unwrap_or(BigInt::zero());
    let modulus_num = BigInt::from_str_radix(modulus, 16).unwrap_or(BigInt::zero());
    
    let result = input_num.modpow(&pubkey_num, &modulus_num);
    format!("{:0>256x}", result)
}

/// Base64 编码
pub fn base64_encode(input: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(input)
}

/// Base64 解码
pub fn base64_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.decode(input)
}

/// Hex 编码
pub fn hex_encode(input: &[u8]) -> String {
    hex::encode(input)
}

/// Hex 解码
pub fn hex_decode(input: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(input)
}
