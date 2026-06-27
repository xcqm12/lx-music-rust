use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 音乐信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MusicInfo {
    pub id: String,
    pub name: String,
    pub singer: Vec<String>,
    pub album_name: String,
    pub interval: u32, // 时长（秒）
    pub source: MusicSource,
    pub quality: BTreeMap<MusicQuality, String>, // 音质 -> ID/Hash (使用 BTreeMap 以支持 Hash)
    pub pic_url: Option<String>,
}

/// 音乐来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MusicSource {
    Kw,  // 酷我
    Kg,  // 酷狗
    Tx,  // QQ音乐
    Wy,  // 网易云
    Mg,  // 咪咕
    Local,
}

impl MusicSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            MusicSource::Kw => "kw",
            MusicSource::Kg => "kg",
            MusicSource::Tx => "tx",
            MusicSource::Wy => "wy",
            MusicSource::Mg => "mg",
            MusicSource::Local => "local",
        }
    }
}

impl TryFrom<&str> for MusicSource {
    type Error = crate::Error;
    
    fn try_from(value: &str) -> crate::Result<Self> {
        match value {
            "kw" => Ok(MusicSource::Kw),
            "kg" => Ok(MusicSource::Kg),
            "tx" => Ok(MusicSource::Tx),
            "wy" => Ok(MusicSource::Wy),
            "mg" => Ok(MusicSource::Mg),
            "local" => Ok(MusicSource::Local),
            _ => Err(crate::Error::SourceNotFound(value.to_string())),
        }
    }
}

/// 音质类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum MusicQuality {
    Lq,   // 低质量 (128k)
    Mq,   // 中等质量 (192k)
    Hq,   // 高质量 (320k)
    Sq,   // 超高质量 (FLAC)
    Hires, // Hi-Res
}

impl MusicQuality {
    pub fn as_str(&self) -> &'static str {
        match self {
            MusicQuality::Lq => "128k",
            MusicQuality::Mq => "192k",
            MusicQuality::Hq => "320k",
            MusicQuality::Sq => "flac",
            MusicQuality::Hires => "hires",
        }
    }
}

/// 歌词信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricInfo {
    pub lyric: String,           // 原歌词 (LRC格式)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlyric: Option<String>,  // 翻译歌词
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rlyric: Option<String>,  // 罗马音歌词
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lxlyric: Option<String>, // 逐字歌词 (LX格式)
}

/// 播放状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayState {
    Idle,
    Playing,
    Paused,
    Stopped,
    Buffering,
    Error,
}

/// 播放模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    Order,     // 顺序播放
    Loop,      // 列表循环
    Random,    // 随机播放
    Single,    // 单曲循环
}

/// 播放进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayProgress {
    pub position: f64,  // 当前位置（秒）
    pub duration: f64,  // 总时长（秒）
    pub buffered: f64,  // 缓冲进度（秒）
}

/// 播放列表项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub music_info: MusicInfo,
    pub url: Option<String>,
}

/// 搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub keyword: String,
    pub source: Option<MusicSource>,
    pub page: u32,
    pub limit: u32,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub list: Vec<MusicInfo>,
    pub total: u32,
    pub source: MusicSource,
}

/// 播放器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub volume: f32,
    pub play_mode: PlayMode,
    pub play_quality: MusicQuality,
    pub audio_offload: bool,
    pub handle_audio_focus: bool,
    pub max_cache_size: u64, // MB
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            volume: 1.0,
            play_mode: PlayMode::Order,
            play_quality: MusicQuality::Hq,
            audio_offload: true,
            handle_audio_focus: true,
            max_cache_size: 1024,
        }
    }
}
