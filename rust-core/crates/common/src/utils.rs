use base64::{Engine as _, engine::general_purpose};
use md5::{Md5, Digest};

/// Base64 编码
pub fn base64_encode(input: &[u8]) -> String {
    general_purpose::STANDARD.encode(input)
}

/// Base64 解码
pub fn base64_decode(input: &str) -> crate::Result<Vec<u8>> {
    general_purpose::STANDARD.decode(input)
        .map_err(|e| crate::Error::ParseFailed(e.to_string()))
}

/// MD5 哈希
pub fn md5_hash(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 生成随机字符串
pub fn random_string(len: usize) -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string().replace("-", "")[..len].to_string()
}

/// 格式化时长为 mm:ss
pub fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// 解析时长字符串
pub fn parse_duration(s: &str) -> crate::Result<u32> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let mins: u32 = parts[0].parse()
            .map_err(|_| crate::Error::ParseFailed("Invalid minute".to_string()))?;
        let secs: u32 = parts[1].parse()
            .map_err(|_| crate::Error::ParseFailed("Invalid second".to_string()))?;
        Ok(mins * 60 + secs)
    } else if parts.len() == 3 {
        let hours: u32 = parts[0].parse()
            .map_err(|_| crate::Error::ParseFailed("Invalid hour".to_string()))?;
        let mins: u32 = parts[1].parse()
            .map_err(|_| crate::Error::ParseFailed("Invalid minute".to_string()))?;
        let secs: u32 = parts[2].parse()
            .map_err(|_| crate::Error::ParseFailed("Invalid second".to_string()))?;
        Ok(hours * 3600 + mins * 60 + secs)
    } else {
        Err(crate::Error::ParseFailed("Invalid duration format".to_string()))
    }
}
