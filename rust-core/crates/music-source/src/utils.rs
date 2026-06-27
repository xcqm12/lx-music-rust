use std::collections::HashMap;

/// 计算字符串相似度 (Levenshtein Distance 简化版)
pub fn similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    
    let len_a = a.chars().count();
    let len_b = b.chars().count();
    let max_len = len_a.max(len_b);
    
    // 简化的相似度计算
    let common: usize = a.chars()
        .zip(b.chars())
        .filter(|(ca, cb)| ca == cb)
        .count();
    
    common as f64 / max_len as f64
}

/// 构建查询字符串
pub fn build_query(params: &HashMap<&str, &str>) -> String {
    params.iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// 移除 HTML 标签
pub fn strip_html(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(html, "").to_string()
}

/// 解析 JSONP 响应
pub fn parse_jsonp(jsonp: &str) -> Option<serde_json::Value> {
    // 提取 callback(json) 中的 json 部分
    let re = regex::Regex::new(r"^[^(]+\((.+)\);?$").unwrap();
    re.captures(jsonp)
        .and_then(|caps| caps.get(1))
        .and_then(|m| serde_json::from_str(m.as_str()).ok())
}

/// 格式化歌手名称
pub fn format_singers(singers: &[String]) -> String {
    singers.join("、")
}

/// 解析时长字符串（秒）
pub fn parse_interval(interval: &str) -> u32 {
    interval.parse().unwrap_or(0)
}

/// 解析时长字符串（支持 "mm:ss" 格式）
pub fn parse_duration(duration: &str) -> Option<u32> {
    let parts: Vec<&str> = duration.split(':').collect();
    match parts.len() {
        1 => duration.parse().ok(),
        2 => {
            let minutes: u32 = parts[0].parse().ok()?;
            let seconds: u32 = parts[1].parse().ok()?;
            Some(minutes * 60 + seconds)
        }
        _ => None,
    }
}

/// 生成随机字符串
pub fn random_string(len: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    
    (0..len)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

/// 时间戳（毫秒）
pub fn timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// 时间戳（秒）
pub fn timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
