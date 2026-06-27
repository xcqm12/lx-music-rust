//! Rust-native music source implementations
//!
//! Implements music search, URL resolution, lyric retrieval, and album art
//! for multiple music platforms (酷我, 酷狗, 咪咕) using pure Rust HTTP requests.
//!
//! QQ音乐 and 网易云音乐 require complex crypto signing and are handled
//! via the JS engine fallback.

use crate::http_utils;
use crate::music_source::{MusicInfo, QualityInfo, SearchResult, LyricInfo, SourceError, Result};
use serde::Deserialize;
use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Pre-compiled regex for HTML entity decoding
static DECODE_NAME_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"&#(\d+);").unwrap()
});

// ============================================================================
// Source trait
// ============================================================================

/// Each music source implements this trait
pub trait MusicSourceApi: Send + Sync {
    /// Get source ID (e.g. "kw", "kg", "mg")
    fn source_id(&self) -> &str;
    /// Get source display name
    fn source_name(&self) -> &str;
    /// Search music by keyword
    fn search(&self, keyword: &str, page: usize, limit: usize) -> Result<SearchResult>;
    /// Get music play URL for given quality
    fn get_music_url(&self, music_info: &MusicInfo, quality: &str) -> Result<String>;
    /// Get lyric for a song
    fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo>;
    /// Get album art URL
    fn get_pic(&self, music_info: &MusicInfo) -> Result<String>;
}

// ============================================================================
// Helper functions
// ============================================================================

/// Decode HTML-encoded names (&#xxx; format)
fn decode_name(name: &str) -> String {
    DECODE_NAME_RE.replace_all(name, |caps: &regex::Captures| {
        let code: u32 = caps[1].parse().unwrap_or(0x3f);
        char::from_u32(code).unwrap_or('?').to_string()
    }).to_string()
}

/// Format seconds to "mm:ss"
fn format_play_time(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// Format file size to human-readable
fn size_format(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / 1048576.0)
    } else if bytes >= 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{}B", bytes)
    }
}

/// Parse size string to bytes
fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    if s.ends_with("MB") {
        s[..s.len()-2].parse::<f64>().ok().map(|v| (v * 1048576.0) as u64)
    } else if s.ends_with("KB") {
        s[..s.len()-2].parse::<f64>().ok().map(|v| (v * 1024.0) as u64)
    } else if s.ends_with("B") {
        s[..s.len()-1].parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

/// HTTP GET request with JSON parsing
fn http_get_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T> {
    let resp = http_utils::get(url).map_err(|e| SourceError::NetworkError(e))?;
    if resp.status != 200 {
        return Err(SourceError::NetworkError(format!("HTTP {}", resp.status)));
    }
    serde_json::from_str(&resp.body).map_err(|e| SourceError::ParseError(e.to_string()))
}

/// HTTP GET request returning raw body
fn http_get_body(url: &str) -> Result<String> {
    let resp = http_utils::get(url).map_err(|e| SourceError::NetworkError(e))?;
    if resp.status != 200 {
        return Err(SourceError::NetworkError(format!("HTTP {}", resp.status)));
    }
    Ok(resp.body)
}

/// Sort singers and join with separator
fn format_singer(singers: &str) -> String {
    let parts: Vec<&str> = singers.split('、').collect();
    let mut sorted = parts.clone();
    sorted.sort();
    sorted.join("、")
}

/// Simple URL encoding
pub(crate) fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for byte in s.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(*byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

// ============================================================================
// Source Manager
// ============================================================================

use std::sync::{Arc, RwLock};

/// Source manager singleton
static SOURCE_MANAGER: Lazy<Arc<RwLock<SourceManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(SourceManager::new()))
});

/// Get the global source manager
pub fn get_source_manager() -> Arc<RwLock<SourceManager>> {
    SOURCE_MANAGER.clone()
}

/// Central source manager
pub struct SourceManager {
    native_sources: HashMap<String, Box<dyn MusicSourceApi>>,
    #[allow(dead_code)]
    js_sources: HashMap<String, String>, // source_id -> source_code
    pub sources_list: Vec<SourceListEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceListEntry {
    pub id: String,
    pub name: String,
    pub is_native: bool,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            native_sources: HashMap::new(),
            js_sources: HashMap::new(),
            sources_list: Vec::new(),
        }
    }

    /// Register a native Rust source
    pub fn register_native(&mut self, source: Box<dyn MusicSourceApi>) {
        let id = source.source_id().to_string();
        let name = source.source_name().to_string();
        self.sources_list.push(SourceListEntry {
            id: id.clone(),
            name: name.clone(),
            is_native: true,
        });
        self.native_sources.insert(id, source);
    }

    /// Register a JS source (for tx, wy that need crypto)
    pub fn register_js(&mut self, id: &str, name: &str) {
        self.sources_list.push(SourceListEntry {
            id: id.to_string(),
            name: name.to_string(),
            is_native: false,
        });
    }

    /// Get all source entries
    pub fn get_source_list(&self) -> Vec<SourceListEntry> {
        self.sources_list.clone()
    }

    /// Check if a source is native
    pub fn is_native(&self, id: &str) -> bool {
        self.native_sources.contains_key(id)
    }

    /// Search using native source
    pub fn native_search(&self, source_id: &str, keyword: &str, page: usize, limit: usize) -> Result<SearchResult> {
        let source = self.native_sources.get(source_id)
            .ok_or_else(|| SourceError::SourceNotFound(source_id.to_string()))?;
        source.search(keyword, page, limit)
    }

    /// Get music URL using native source
    pub fn native_get_music_url(&self, source_id: &str, music_info: &MusicInfo, quality: &str) -> Result<String> {
        let source = self.native_sources.get(source_id)
            .ok_or_else(|| SourceError::SourceNotFound(source_id.to_string()))?;
        source.get_music_url(music_info, quality)
    }

    /// Get lyric using native source
    pub fn native_get_lyric(&self, source_id: &str, music_info: &MusicInfo) -> Result<LyricInfo> {
        let source = self.native_sources.get(source_id)
            .ok_or_else(|| SourceError::SourceNotFound(source_id.to_string()))?;
        source.get_lyric(music_info)
    }

    /// Get pic using native source
    pub fn native_get_pic(&self, source_id: &str, music_info: &MusicInfo) -> Result<String> {
        let source = self.native_sources.get(source_id)
            .ok_or_else(|| SourceError::SourceNotFound(source_id.to_string()))?;
        source.get_pic(music_info)
    }
}

pub mod kw;
pub mod kg;
pub mod mg;

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Helper function tests
    // ========================================================================
    #[test]
    fn test_decode_name() {
        assert_eq!(decode_name("&#x4f60;&#x597d;"), "&#x4f60;&#x597d;");
        assert_eq!(decode_name("&#25105;&#22909;"), "我好");
        assert_eq!(decode_name("hello"), "hello");
        assert_eq!(decode_name(""), "");
    }

    #[test]
    fn test_format_play_time() {
        assert_eq!(format_play_time(0.0), "00:00");
        assert_eq!(format_play_time(60.0), "01:00");
        assert_eq!(format_play_time(90.0), "01:30");
        assert_eq!(format_play_time(3661.0), "61:01");
    }

    #[test]
    fn test_size_format() {
        assert_eq!(size_format(5), "5B");
        assert_eq!(size_format(1024), "1.0KB");
        assert_eq!(size_format(1048576), "1.0MB");
        assert_eq!(size_format(5242880), "5.0MB");
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("5B"), Some(5));
        assert_eq!(parse_size("1KB"), Some(1024));
        assert_eq!(parse_size("1.5MB"), Some(1572864));
        assert_eq!(parse_size("100"), Some(100));
        assert_eq!(parse_size("invalid"), None);
    }

    #[test]
    fn test_format_singer() {
        assert_eq!(format_singer("歌手A、歌手B"), "歌手A、歌手B");
        // Sorted by name
        assert_eq!(format_singer("B、A"), "A、B");
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("abc123"), "abc123");
        assert_eq!(urlencoding("你好"), "%E4%BD%A0%E5%A5%BD");
        assert_eq!(urlencoding("test@#$"), "test%40%23%24");
    }

    // ========================================================================
    // SourceManager tests
    // ========================================================================
    #[test]
    fn test_source_manager_new() {
        let manager = SourceManager::new();
        assert!(manager.get_source_list().is_empty());
    }

    #[test]
    fn test_register_native() {
        let mut manager = SourceManager::new();
        manager.register_native(Box::new(kw::KwSource::new()));
        let list = manager.get_source_list();
        assert_eq!(list.len(), 1);
        assert!(list[0].is_native);
        assert_eq!(list[0].id, "kw");
    }

    #[test]
    fn test_register_js() {
        let mut manager = SourceManager::new();
        manager.register_js("tx", "QQ音乐");
        manager.register_js("wy", "网易云音乐");
        let list = manager.get_source_list();
        assert_eq!(list.len(), 2);
        assert!(!list[0].is_native);
        assert!(!list[1].is_native);
    }

    #[test]
    fn test_is_native() {
        let mut manager = SourceManager::new();
        manager.register_native(Box::new(kw::KwSource::new()));
        manager.register_js("tx", "QQ音乐");
        assert!(manager.is_native("kw"));
        assert!(!manager.is_native("tx"));
        assert!(!manager.is_native("nonexistent"));
    }

    #[test]
    fn test_source_list_entry() {
        let entry = SourceListEntry {
            id: "kw".to_string(),
            name: "酷我".to_string(),
            is_native: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: SourceListEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "kw");
        assert_eq!(parsed.name, "酷我");
        assert!(parsed.is_native);
    }

    #[test]
    fn test_source_manager_multiple_native() {
        let mut manager = SourceManager::new();
        manager.register_native(Box::new(kw::KwSource::new()));
        manager.register_native(Box::new(kg::KgSource::new()));
        manager.register_native(Box::new(mg::MgSource::new()));
        let list = manager.get_source_list();
        assert_eq!(list.len(), 3);
        assert!(manager.is_native("kw"));
        assert!(manager.is_native("kg"));
        assert!(manager.is_native("mg"));
    }
}