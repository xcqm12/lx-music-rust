pub mod parser;
pub mod sync;
pub mod manager;
pub mod types;

use common::{LyricInfo, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

pub use parser::LyricParser;
pub use sync::LyricSync;
pub use types::{LyricLine, ParsedLyric, WordTiming};

/// 歌词管理器
pub struct LyricManager {
    parser: LyricParser,
    current_lyric: Arc<RwLock<Option<ParsedLyric>>>,
    event_sender: tokio::sync::mpsc::Sender<LyricEvent>,
}

/// 歌词事件
#[derive(Debug, Clone)]
pub enum LyricEvent {
    /// 歌词加载完成
    Loaded(ParsedLyric),
    /// 当前行变化
    LineChanged(usize, LyricLine),
    /// 当前字变化（逐字歌词）
    WordChanged(usize, usize, f64),
    /// 歌词错误
    Error(String),
}

impl LyricManager {
    pub fn new(event_sender: tokio::sync::mpsc::Sender<LyricEvent>) -> Self {
        Self {
            parser: LyricParser::new(),
            current_lyric: Arc::new(RwLock::new(None)),
            event_sender,
        }
    }
    
    /// 加载歌词
    pub async fn load_lyric(&self, lyric_info: LyricInfo) -> Result<()> {
        let parsed = self.parser.parse(&lyric_info)?;
        
        // 发送加载完成事件
        let _ = self.event_sender.send(LyricEvent::Loaded(parsed.clone())).await;
        
        // 保存当前歌词
        *self.current_lyric.write().await = Some(parsed);
        
        Ok(())
    }
    
    /// 获取当前歌词
    pub async fn get_current_lyric(&self) -> Option<ParsedLyric> {
        self.current_lyric.read().await.clone()
    }
    
    /// 根据时间获取当前行索引
    pub async fn get_current_line_index(&self, time_ms: f64) -> Option<usize> {
        let lyric = self.current_lyric.read().await;
        lyric.as_ref().map(|l| l.find_current_line(time_ms))
    }
    
    /// 根据时间获取当前行
    pub async fn get_current_line(&self, time_ms: f64) -> Option<LyricLine> {
        let index = self.get_current_line_index(time_ms).await?;
        let lyric = self.current_lyric.read().await;
        lyric.as_ref()?.lines.get(index).cloned()
    }
    
    /// 获取翻译
    pub async fn get_translation(&self, time_ms: f64) -> Option<String> {
        let lyric = self.current_lyric.read().await;
        let parsed = lyric.as_ref()?;
        let index = parsed.find_current_line(time_ms);
        parsed.lines.get(index).and_then(|l| l.translation.clone())
    }
    
    /// 获取罗马音
    pub async fn get_romaji(&self, time_ms: f64) -> Option<String> {
        let lyric = self.current_lyric.read().await;
        let parsed = lyric.as_ref()?;
        let index = parsed.find_current_line(time_ms);
        parsed.lines.get(index).and_then(|l| l.romaji.clone())
    }
    
    /// 清空歌词
    pub async fn clear(&self) {
        *self.current_lyric.write().await = None;
    }
    
    /// 解析外部歌词文件
    pub fn parse_file(&self, content: &str, format: LyricFormat) -> Result<ParsedLyric> {
        self.parser.parse_with_format(content, format)
    }
}

/// 歌词格式
#[derive(Debug, Clone, Copy)]
pub enum LyricFormat {
    /// LRC 格式
    Lrc,
    /// KRC 格式（酷狗加密）
    Krc,
    /// QRC 格式（QQ音乐）
    Qrc,
    /// TTML 格式
    Ttml,
    /// YRC 格式（网易云逐字）
    Yrc,
    /// 未知格式（自动检测）
    Auto,
}

impl LyricFormat {
    /// 根据内容检测格式
    pub fn detect(content: &str) -> Self {
        if content.starts_with("[krc]") || content.starts_with("[id:") {
            LyricFormat::Krc
        } else if content.contains("<tt") {
            LyricFormat::Ttml
        } else if content.contains("yrc") || content.contains("[155000,") {
            LyricFormat::Yrc
        } else if content.starts_with("<?xml") && content.contains("<QrcInfos>") {
            LyricFormat::Qrc
        } else if content.contains('[') && content.contains(']') {
            LyricFormat::Lrc
        } else {
            LyricFormat::Auto
        }
    }
}
