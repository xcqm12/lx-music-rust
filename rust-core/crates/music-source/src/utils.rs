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

/// 版本特征词条，与上游 `src/utils/musicSdk/versionChars.ts` 保持一致
/// （伴奏/现场/翻唱/混音等版本标识，跨源匹配时两侧必须一致）
pub const VERSION_CHARS: &[&str] = &[
    // 伴奏 / 无人声
    "instrumental",
    "inst",
    "karaoke",
    "backing track",
    "minus one",
    "minus vocal",
    "no vocal",
    "off vocal",
    "vocal removed",
    "伴奏",
    "伴唱",
    "卡拉ok",
    "纯音乐",
    "无人声",
    "去人声",
    "消音",
    "オフボーカル",
    "インスト",
    "반주",
    "노래방",
    // 现场
    "live",
    "concert",
    "tour",
    "festival",
    "stage",
    "session",
    "现场",
    "演唱会",
    "巡演",
    "音乐会",
    "ライブ",
    "공연",
    "라이브",
    // 翻唱
    "cover",
    "tribute",
    "rendition",
    "remake",
    "翻唱",
    "致敬",
    "改编",
    "カバー",
    "커버",
    // 混音
    "remix",
    "mix",
    "club",
    "dance",
    "extended",
    "house",
    "edm",
    "dj",
    "dub",
    "trap",
    "混音",
    "重混",
    "电音",
    "リミックス",
    "리믹스",
    // 原声 / 不插电
    "acoustic",
    "unplugged",
    "stripped",
    "原声",
    "不插电",
    "アコースティック",
    "어쿠스틱",
    // Demo
    "demo",
    "小样",
    "试听",
    "デモ",
    "데모",
    // 速度变化
    "slowed",
    "slowed reverb",
    "sped up",
    "speed up",
    "nightcore",
    "慢速",
    "加速",
    "变速",
    "ナイトコア",
    // 特殊乐器版
    "piano",
    "guitar",
    "orchestra",
    "symphony",
    "violin",
    "钢琴",
    "吉他",
    "管弦乐",
    "小提琴",
    "ピアノ",
    "ギター",
    "피아노",
    "기타",
];

/// 与上游 `matchMusicInfo` 的 `filterStr` 同口径：
/// 去除空白与分隔符（` '|.，&"、()（）`~<>/[]|!！`）后转小写
pub fn filter_str_lower(s: &str) -> String {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"[\s'\.,，&"、()（）`~\-<>/\[\]!！|]"#).unwrap()
    });
    re.replace_all(s, "").to_lowercase()
}

/// 与上游 `isEqualsVersionMusicNameChar` 同口径：
/// 两个名字对每个版本特征词条的包含结果必须完全一致，否则视为不同版本
pub fn is_equals_version_chars(a: &str, b: &str) -> bool {
    VERSION_CHARS.iter().all(|c| a.contains(c) == b.contains(c))
}

/// HTML 实体解码（对应上游 `decodeName` = he.decode 的常用子集）
pub fn decode_html_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let re = regex::Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z][a-zA-Z0-9]*);").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        let ent = &caps[1];
        if let Some(hex) = ent.strip_prefix("#x").or_else(|| ent.strip_prefix("#X")) {
            u32::from_str_radix(hex, 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| caps[0].to_string())
        } else if let Some(dec) = ent.strip_prefix('#') {
            dec.parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| caps[0].to_string())
        } else {
            match ent {
                "amp" => "&".to_string(),
                "lt" => "<".to_string(),
                "gt" => ">".to_string(),
                "quot" => "\"".to_string(),
                "apos" => "'".to_string(),
                "nbsp" => "\u{a0}".to_string(),
                _ => caps[0].to_string(),
            }
        }
    })
    .to_string()
}
