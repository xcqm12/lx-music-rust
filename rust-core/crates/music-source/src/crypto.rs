/// MD5 哈希
pub fn md5(input: &str) -> String {
    let digest = md5::compute(input.as_bytes());
    format!("{:x}", digest)
}

/// MD5 哈希（大写）
pub fn md5_upper(input: &str) -> String {
    md5(input).to_uppercase()
}

/// PKCS7 填充
fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let padding_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    for _ in 0..padding_len {
        padded.push(padding_len as u8);
    }
    padded
}

/// PKCS7 填充验证和解码
fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("Empty data".to_string());
    }
    let padding_len = *data.last().unwrap() as usize;
    if padding_len > data.len() {
        return Err("Invalid padding".to_string());
    }
    Ok(data[..data.len() - padding_len].to_vec())
}

/// AES ECB 加密
pub fn aes_ecb_encrypt(plaintext: &[u8], key: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::BlockEncrypt;
    use aes::cipher::KeyInit;
    
    let cipher = Aes128::new_from_slice(key).unwrap();
    let padded = pkcs7_pad(plaintext, 16);
    let mut result = Vec::new();
    
    for block in padded.chunks(16) {
        let mut block_arr = [0u8; 16];
        block_arr.copy_from_slice(block);
        cipher.encrypt_block((&mut block_arr).into());
        result.extend_from_slice(&block_arr);
    }
    
    result
}

/// AES ECB 解密
pub fn aes_ecb_decrypt(ciphertext: &[u8], key: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::BlockDecrypt;
    use aes::cipher::KeyInit;
    
    let cipher = Aes128::new_from_slice(key).unwrap();
    let mut result = Vec::new();
    
    for block in ciphertext.chunks(16) {
        let mut block_arr = [0u8; 16];
        block_arr.copy_from_slice(block);
        cipher.decrypt_block((&mut block_arr).into());
        result.extend_from_slice(&block_arr);
    }
    
    pkcs7_unpad(&result).unwrap_or_default()
}

/// AES CBC 加密
pub fn aes_cbc_encrypt(plaintext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::BlockEncrypt;
    use aes::cipher::KeyInit;
    
    let cipher = Aes128::new_from_slice(key).unwrap();
    let padded = pkcs7_pad(plaintext, 16);
    let mut result = Vec::new();
    let mut prev_block = iv.to_vec();
    
    for block in padded.chunks(16) {
        let mut block_arr = [0u8; 16];
        // XOR with previous ciphertext block
        for (i, b) in block.iter().enumerate() {
            block_arr[i] = b ^ prev_block[i];
        }
        cipher.encrypt_block((&mut block_arr).into());
        result.extend_from_slice(&block_arr);
        prev_block = block_arr.to_vec();
    }
    
    result
}

/// AES CBC 解密
pub fn aes_cbc_decrypt(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    use aes::Aes128;
    use aes::cipher::BlockDecrypt;
    use aes::cipher::KeyInit;
    
    let cipher = Aes128::new_from_slice(key).unwrap();
    let mut result = Vec::new();
    let mut prev_block = iv.to_vec();
    
    for block in ciphertext.chunks(16) {
        let mut block_arr = [0u8; 16];
        block_arr.copy_from_slice(block);
        cipher.decrypt_block((&mut block_arr).into());
        // XOR with previous ciphertext block
        for (i, b) in block_arr.iter_mut().enumerate() {
            *b ^= prev_block[i];
        }
        result.extend_from_slice(&block_arr);
        prev_block = block.to_vec();
    }
    
    pkcs7_unpad(&result).unwrap_or_default()
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
