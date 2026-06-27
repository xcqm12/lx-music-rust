use serde::{Deserialize, Serialize};

/// 解析后的歌词
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLyric {
    /// 标题
    pub title: Option<String>,
    /// 艺术家
    pub artist: Option<String>,
    /// 专辑
    pub album: Option<String>,
    /// 歌词作者
    pub lyricist: Option<String>,
    /// 时长
    pub length: Option<f64>,
    /// 偏移量
    pub offset: f64,
    /// 歌词行
    pub lines: Vec<LyricLine>,
    /// 是否包含逐字信息
    pub has_word_timing: bool,
}

impl ParsedLyric {
    /// 创建空歌词
    pub fn empty() -> Self {
        Self {
            title: None,
            artist: None,
            album: None,
            lyricist: None,
            length: None,
            offset: 0.0,
            lines: Vec::new(),
            has_word_timing: false,
        }
    }
    
    /// 根据时间获取当前行索引
    pub fn find_current_line(&self, time_ms: f64) -> usize {
        let time_with_offset = time_ms + self.offset;
        
        // 二分查找
        let mut left = 0;
        let mut right = self.lines.len();
        
        while left < right {
            let mid = (left + right) / 2;
            if let Some(line) = self.lines.get(mid) {
                if line.start_time <= time_with_offset {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            } else {
                break;
            }
        }
        
        if left > 0 {
            left - 1
        } else {
            0
        }
    }
    
    /// 获取某行的歌词文本
    pub fn get_text(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(|l| l.text.as_str())
    }
    
    /// 获取总行数
    pub fn len(&self) -> usize {
        self.lines.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// 歌词行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricLine {
    /// 开始时间（毫秒）
    pub start_time: f64,
    /// 持续时间（毫秒）
    pub duration: f64,
    /// 歌词文本
    pub text: String,
    /// 翻译
    pub translation: Option<String>,
    /// 罗马音
    pub romaji: Option<String>,
    /// 逐字时间信息
    pub words: Vec<WordTiming>,
}

impl LyricLine {
    /// 创建新行
    pub fn new(start_time: f64, text: impl Into<String>) -> Self {
        Self {
            start_time,
            duration: 0.0,
            text: text.into(),
            translation: None,
            romaji: None,
            words: Vec::new(),
        }
    }
    
    /// 设置持续时间
    pub fn with_duration(mut self, duration: f64) -> Self {
        self.duration = duration;
        self
    }
    
    /// 设置翻译
    pub fn with_translation(mut self, translation: impl Into<String>) -> Self {
        self.translation = Some(translation.into());
        self
    }
    
    /// 设置罗马音
    pub fn with_romaji(mut self, romaji: impl Into<String>) -> Self {
        self.romaji = Some(romaji.into());
        self
    }
    
    /// 添加逐字信息
    pub fn with_words(mut self, words: Vec<WordTiming>) -> Self {
        self.words = words;
        self
    }
    
    /// 获取结束时间
    pub fn end_time(&self) -> f64 {
        self.start_time + self.duration
    }
    
    /// 是否包含逐字信息
    pub fn has_word_timing(&self) -> bool {
        !self.words.is_empty()
    }
    
    /// 根据时间获取当前字索引
    pub fn find_current_word(&self, time_ms: f64) -> Option<usize> {
        if self.words.is_empty() {
            return None;
        }
        
        let relative_time = time_ms - self.start_time;
        
        for (i, word) in self.words.iter().enumerate() {
            if relative_time >= word.start_time 
                && relative_time < word.start_time + word.duration {
                return Some(i);
            }
        }
        
        None
    }
}

/// 逐字时间信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTiming {
    /// 字在歌词中的起始位置（字符索引）
    pub start_char: usize,
    /// 字的长度（字符数）
    pub char_count: usize,
    /// 开始时间（毫秒）
    pub start_time: f64,
    /// 持续时间（毫秒）
    pub duration: f64,
    /// 字内容
    pub text: String,
}

impl WordTiming {
    /// 创建逐字信息
    pub fn new(
        start_char: usize,
        char_count: usize,
        start_time: f64,
        duration: f64,
        text: impl Into<String>,
    ) -> Self {
        Self {
            start_char,
            char_count,
            start_time,
            duration,
            text: text.into(),
        }
    }
    
    /// 获取结束时间
    pub fn end_time(&self) -> f64 {
        self.start_time + self.duration
    }
}

/// 元数据标签
#[derive(Debug, Clone)]
pub enum LyricTag {
    Title(String),
    Artist(String),
    Album(String),
    Author(String),
    Length(f64),
    Offset(f64),
    By(String),
    Version(String),
    Tool(String),
    Unknown(String, String),
}

impl LyricTag {
    /// 从 LRC 标签解析
    pub fn from_lrc(key: &str, value: &str) -> Self {
        match key.to_lowercase().as_str() {
            "ti" => LyricTag::Title(value.to_string()),
            "ar" => LyricTag::Artist(value.to_string()),
            "al" => LyricTag::Album(value.to_string()),
            "au" => LyricTag::Author(value.to_string()),
            "length" => {
                let ms = parse_duration(value).unwrap_or(0.0);
                LyricTag::Length(ms)
            }
            "offset" => {
                let offset = value.parse::<f64>().unwrap_or(0.0);
                LyricTag::Offset(offset)
            }
            "by" => LyricTag::By(value.to_string()),
            "version" => LyricTag::Version(value.to_string()),
            "tool" => LyricTag::Tool(value.to_string()),
            _ => LyricTag::Unknown(key.to_string(), value.to_string()),
        }
    }
}

/// 解析时长字符串为毫秒
fn parse_duration(s: &str) -> Option<f64> {
    // 格式: mm:ss.xx 或 mm:ss:xx
    let parts: Vec<&str> = s.split([':', '.']).collect();
    if parts.len() >= 2 {
        let mins: f64 = parts[0].parse().ok()?;
        let secs: f64 = parts[1].parse().ok()?;
        let ms = if parts.len() > 2 {
            parts[2].parse::<f64>().unwrap_or(0.0) * 10.0
        } else {
            0.0
        };
        Some((mins * 60.0 + secs) * 1000.0 + ms)
    } else {
        None
    }
}
