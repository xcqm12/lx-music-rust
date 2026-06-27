//! Music Source Management Module
//! 
//! Handles music source registration, music search, URL resolution, 
//! and metadata retrieval from various music APIs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SourceError {
    #[error("Source not found: {0}")]
    SourceNotFound(String),
    #[error("Search failed: {0}")]
    SearchFailed(String),
    #[error("URL resolution failed: {0}")]
    UrlResolutionFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
}

pub type Result<T> = std::result::Result<T, SourceError>;

/// Music source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub source_type: SourceType,
}

/// Source type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Built-in official sources
    Builtin,
    /// Custom user-added sources
    Custom,
}

/// Music quality information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityInfo {
    pub quality: String,
    pub size: Option<String>,
    pub url: Option<String>,
}

/// Music information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicInfo {
    pub id: String,
    pub name: String,
    pub singer: String,
    pub source: String,
    #[serde(rename = "albumId")]
    pub album_id: Option<String>,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    pub duration: Option<String>,
    #[serde(rename = "picUrl")]
    pub pic_url: Option<String>,
    #[serde(rename = "lrcUrl")]
    pub lrc_url: Option<String>,
    #[serde(default)]
    pub qualitys: Vec<QualityInfo>,
    pub url: Option<String>,
}

/// Search result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub source: String,
    pub keyword: String,
    pub data: Vec<MusicInfo>,
    #[serde(rename = "totalCount")]
    pub total_count: usize,
    #[serde(rename = "pageSize")]
    pub page_size: usize,
    #[serde(rename = "pageIndex")]
    pub page_index: usize,
}

/// Lyric information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricInfo {
    #[serde(rename = "lrc")]
    pub lyric: Option<String>,
    #[serde(rename = "lrcT")]
    pub translation: Option<String>,
    #[serde(rename = "lrcRoma")]
    pub romaji: Option<String>,
    #[serde(rename = "trc")]
    pub raw_translation: Option<String>,
}

/// Download task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub url: String,
    pub filename: String,
    pub size: u64,
}

/// Download status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
    Cancelled,
}

/// Global source registry
static SOURCE_REGISTRY: Lazy<Arc<RwLock<HashMap<String, SourceInfo>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

/// Music source manager
pub struct MusicSourceManager;

impl MusicSourceManager {
    /// Register a new music source
    pub fn register_source(id: &str, name: &str, source_type: SourceType) -> Result<()> {
        let info = SourceInfo {
            id: id.to_string(),
            name: name.to_string(),
            enabled: true,
            source_type,
        };
        
        let mut registry = SOURCE_REGISTRY.write()
            .map_err(|_| SourceError::ParseError("Failed to acquire write lock".to_string()))?;
        
        registry.insert(id.to_string(), info);
        Ok(())
    }
    
    /// Get all registered sources
    pub fn get_sources() -> Vec<SourceInfo> {
        SOURCE_REGISTRY.read()
            .map(|r| r.values().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get source by ID
    pub fn get_source(id: &str) -> Option<SourceInfo> {
        SOURCE_REGISTRY.read()
            .ok()
            .and_then(|r| r.get(id).cloned())
    }
    
    /// Enable/disable source
    pub fn set_source_enabled(id: &str, enabled: bool) -> Result<()> {
        let mut registry = SOURCE_REGISTRY.write()
            .map_err(|_| SourceError::ParseError("Failed to acquire write lock".to_string()))?;
        
        if let Some(source) = registry.get_mut(id) {
            source.enabled = enabled;
            Ok(())
        } else {
            Err(SourceError::SourceNotFound(id.to_string()))
        }
    }
    
    /// Remove a source
    pub fn remove_source(id: &str) -> Result<()> {
        let mut registry = SOURCE_REGISTRY.write()
            .map_err(|_| SourceError::ParseError("Failed to acquire write lock".to_string()))?;
        
        if registry.remove(id).is_some() {
            Ok(())
        } else {
            Err(SourceError::SourceNotFound(id.to_string()))
        }
    }
    
    /// Parse music info from JSON string
    pub fn parse_music_info(json: &str) -> Result<MusicInfo> {
        serde_json::from_str(json)
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }
    
    /// Parse search result from JSON string
    pub fn parse_search_result(json: &str) -> Result<SearchResult> {
        serde_json::from_str(json)
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }
    
    /// Serialize music info to JSON
    pub fn serialize_music_info(info: &MusicInfo) -> Result<String> {
        serde_json::to_string(info)
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }
    
    /// Serialize list of music info to JSON
    pub fn serialize_music_list(list: &[MusicInfo]) -> Result<String> {
        serde_json::to_string(list)
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_info_creation() {
        let info = SourceInfo {
            id: "kw".to_string(),
            name: "酷我".to_string(),
            enabled: true,
            source_type: SourceType::Builtin,
        };
        assert_eq!(info.id, "kw");
        assert_eq!(info.name, "酷我");
        assert!(info.enabled);
        assert_eq!(info.source_type, SourceType::Builtin);
    }

    #[test]
    fn test_source_type_serialization() {
        let builtin = SourceType::Builtin;
        let custom = SourceType::Custom;
        let json_builtin = serde_json::to_string(&builtin).unwrap();
        let json_custom = serde_json::to_string(&custom).unwrap();
        assert!(json_builtin.contains("builtin"));
        assert!(json_custom.contains("custom"));
    }

    #[test]
    fn test_music_info_creation() {
        let info = MusicInfo {
            id: "123".to_string(),
            name: "测试歌曲".to_string(),
            singer: "测试歌手".to_string(),
            source: "kw".to_string(),
            album_id: Some("album1".to_string()),
            album_name: Some("测试专辑".to_string()),
            duration: Some("04:30".to_string()),
            pic_url: Some("http://example.com/pic.jpg".to_string()),
            lrc_url: None,
            qualitys: vec![],
            url: None,
        };
        assert_eq!(info.id, "123");
        assert_eq!(info.name, "测试歌曲");
        assert_eq!(info.singer, "测试歌手");
        assert_eq!(info.source, "kw");
        assert_eq!(info.album_id, Some("album1".to_string()));
        assert_eq!(info.album_name, Some("测试专辑".to_string()));
        assert_eq!(info.duration, Some("04:30".to_string()));
        assert!(info.pic_url.is_some());
        assert!(info.lrc_url.is_none());
        assert!(info.qualitys.is_empty());
        assert!(info.url.is_none());
    }

    #[test]
    fn test_music_info_json_serialization() {
        let info = MusicInfo {
            id: "123".to_string(),
            name: "song".to_string(),
            singer: "artist".to_string(),
            source: "kw".to_string(),
            album_id: None,
            album_name: None,
            duration: None,
            pic_url: None,
            lrc_url: None,
            qualitys: vec![],
            url: Some("http://example.com/music.mp3".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: MusicInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "123");
        assert_eq!(parsed.name, "song");
        assert_eq!(parsed.url, Some("http://example.com/music.mp3".to_string()));
    }

    #[test]
    fn test_quality_info() {
        let qi = QualityInfo {
            quality: "320k".to_string(),
            size: Some("10.5MB".to_string()),
            url: Some("http://example.com/music.mp3".to_string()),
        };
        assert_eq!(qi.quality, "320k");
        assert_eq!(qi.size, Some("10.5MB".to_string()));
        assert_eq!(qi.url, Some("http://example.com/music.mp3".to_string()));
    }

    #[test]
    fn test_search_result() {
        let result = SearchResult {
            source: "kw".to_string(),
            keyword: "测试".to_string(),
            data: vec![],
            total_count: 100,
            page_size: 20,
            page_index: 1,
        };
        assert_eq!(result.source, "kw");
        assert_eq!(result.keyword, "测试");
        assert_eq!(result.total_count, 100);
        assert_eq!(result.page_size, 20);
        assert_eq!(result.page_index, 1);
        assert!(result.data.is_empty());
    }

    #[test]
    fn test_lyric_info() {
        let info = LyricInfo {
            lyric: Some("[00:01.00]测试歌词".to_string()),
            translation: Some("[00:01.00]test lyric".to_string()),
            romaji: None,
            raw_translation: None,
        };
        assert!(info.lyric.is_some());
        assert!(info.translation.is_some());
        assert!(info.romaji.is_none());
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("lrc"));
        assert!(json.contains("lrcT"));
    }

    #[test]
    fn test_download_task() {
        let task = DownloadTask {
            url: "http://example.com/music.mp3".to_string(),
            filename: "music.mp3".to_string(),
            size: 10485760,
        };
        assert_eq!(task.url, "http://example.com/music.mp3");
        assert_eq!(task.filename, "music.mp3");
        assert_eq!(task.size, 10485760);
    }

    #[test]
    fn test_download_status() {
        let statuses = vec![
            DownloadStatus::Pending,
            DownloadStatus::Downloading,
            DownloadStatus::Completed,
            DownloadStatus::Failed,
            DownloadStatus::Cancelled,
        ];
        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            let parsed: DownloadStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*status, parsed);
        }
    }

    #[test]
    fn test_source_manager_register() {
        let result = MusicSourceManager::register_source("test_src", "测试源", SourceType::Custom);
        assert!(result.is_ok());
        let source = MusicSourceManager::get_source("test_src");
        assert!(source.is_some());
        assert_eq!(source.unwrap().name, "测试源");
    }

    #[test]
    fn test_source_manager_get_not_found() {
        let source = MusicSourceManager::get_source("nonexistent");
        assert!(source.is_none());
    }

    #[test]
    fn test_source_manager_get_all() {
        MusicSourceManager::register_source("src_a", "A", SourceType::Builtin).ok();
        MusicSourceManager::register_source("src_b", "B", SourceType::Custom).ok();
        let sources = MusicSourceManager::get_sources();
        assert!(sources.len() >= 2);
    }

    #[test]
    fn test_source_manager_set_enabled() {
        MusicSourceManager::register_source("toggle_src", "Toggle", SourceType::Builtin).ok();
        let result = MusicSourceManager::set_source_enabled("toggle_src", false);
        assert!(result.is_ok());
        let source = MusicSourceManager::get_source("toggle_src").unwrap();
        assert!(!source.enabled);

        MusicSourceManager::set_source_enabled("toggle_src", true).ok();
        let source = MusicSourceManager::get_source("toggle_src").unwrap();
        assert!(source.enabled);
    }

    #[test]
    fn test_source_manager_set_enabled_not_found() {
        let result = MusicSourceManager::set_source_enabled("ghost", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_source_manager_remove() {
        MusicSourceManager::register_source("rm_src", "Remove", SourceType::Custom).ok();
        let result = MusicSourceManager::remove_source("rm_src");
        assert!(result.is_ok());
        assert!(MusicSourceManager::get_source("rm_src").is_none());
    }

    #[test]
    fn test_source_manager_remove_not_found() {
        let result = MusicSourceManager::remove_source("ghost2");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_music_info() {
        let json = r#"{"id":"123","name":"song","singer":"artist","source":"kw"}"#;
        let result = MusicSourceManager::parse_music_info(json);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.id, "123");
        assert_eq!(info.name, "song");
    }

    #[test]
    fn test_parse_music_info_invalid() {
        let result = MusicSourceManager::parse_music_info("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_music_info() {
        let info = MusicInfo {
            id: "1".to_string(),
            name: "song".to_string(),
            singer: "artist".to_string(),
            source: "kw".to_string(),
            album_id: None,
            album_name: None,
            duration: None,
            pic_url: None,
            lrc_url: None,
            qualitys: vec![],
            url: None,
        };
        let json = MusicSourceManager::serialize_music_info(&info).unwrap();
        assert!(json.contains("\"id\":\"1\""));
        assert!(json.contains("\"name\":\"song\""));
    }

    #[test]
    fn test_serialize_music_list() {
        let list = vec![
            MusicInfo {
                id: "1".to_string(),
                name: "a".to_string(),
                singer: "x".to_string(),
                source: "kw".to_string(),
                album_id: None,
                album_name: None,
                duration: None,
                pic_url: None,
                lrc_url: None,
                qualitys: vec![],
                url: None,
            },
            MusicInfo {
                id: "2".to_string(),
                name: "b".to_string(),
                singer: "y".to_string(),
                source: "kw".to_string(),
                album_id: None,
                album_name: None,
                duration: None,
                pic_url: None,
                lrc_url: None,
                qualitys: vec![],
                url: None,
            },
        ];
        let json = MusicSourceManager::serialize_music_list(&list).unwrap();
        assert!(json.contains("\"id\":\"1\""));
        assert!(json.contains("\"id\":\"2\""));
    }

    #[test]
    fn test_source_error_display() {
        let err = SourceError::SourceNotFound("kw".to_string());
        assert!(format!("{}", err).contains("kw"));

        let err = SourceError::NetworkError("timeout".to_string());
        assert!(format!("{}", err).contains("timeout"));

        let err = SourceError::ParseError("invalid json".to_string());
        assert!(format!("{}", err).contains("invalid json"));
    }
}