use crate::types::{LyricLine, LyricTag, ParsedLyric, WordTiming};
use crate::LyricFormat;
use common::{LyricInfo, Result, Error};
use regex::Regex;

/// 歌词解析器
pub struct LyricParser {
    time_regex: Regex,
    tag_regex: Regex,
    word_time_regex: Regex,
}

impl LyricParser {
    pub fn new() -> Self {
        Self {
            // 匹配 [mm:ss.xx] 或 [mm:ss:xx] 格式
            time_regex: Regex::new(r"\[(\d{2}):(\d{2})[.:](\d{2,3})\]").unwrap(),
            // 匹配 [key:value] 标签
            tag_regex: Regex::new(r"\[(\w+):([^\]]+)\]").unwrap(),
            // 匹配逐字时间 <start,duration>
            word_time_regex: Regex::new(r"<(\d+),(\d+),(\d+)>").unwrap(),
        }
    }
    
    /// 解析 LyricInfo
    pub fn parse(&self, lyric_info: &LyricInfo) -> Result<ParsedLyric> {
        let mut result = ParsedLyric::empty();
        
        // 解析主歌词
        if !lyric_info.lyric.is_empty() {
            let main_lyric = self.parse_lrc(&lyric_info.lyric)?;
            result = main_lyric;
        }
        
        // 解析翻译歌词
        if let Some(ref tlyric) = lyric_info.tlyric {
            let translation = self.parse_lrc(tlyric)?;
            self.merge_translation(&mut result, &translation);
        }
        
        // 解析罗马音歌词
        if let Some(ref rlyric) = lyric_info.rlyric {
            let romaji = self.parse_lrc(rlyric)?;
            self.merge_romaji(&mut result, &romaji);
        }
        
        // 解析逐字歌词
        if let Some(ref lxlyric) = lyric_info.lxlyric {
            self.parse_word_timing(&mut result, lxlyric)?;
        }
        
        Ok(result)
    }
    
    /// 解析 LRC 格式
    pub fn parse_lrc(&self, content: &str) -> Result<ParsedLyric> {
        let mut lyric = ParsedLyric::empty();
        let mut temp_lines: Vec<(f64, String)> = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // 解析标签 [tag:value]
            for cap in self.tag_regex.captures_iter(line) {
                let key = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let value = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                
                // 尝试作为时间标签解析
                if let Some(time) = self.parse_time_tag(key, value) {
                    temp_lines.push((time, self.extract_text(line)));
                } else {
                    // 元数据标签
                    match LyricTag::from_lrc(key, value) {
                        LyricTag::Title(v) => lyric.title = Some(v),
                        LyricTag::Artist(v) => lyric.artist = Some(v),
                        LyricTag::Album(v) => lyric.album = Some(v),
                        LyricTag::Offset(v) => lyric.offset = v,
                        LyricTag::Length(v) => lyric.length = Some(v),
                        _ => {}
                    }
                }
            }
        }
        
        // 按时间排序
        temp_lines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // 计算每行持续时间
        for i in 0..temp_lines.len() {
            let (start, text) = &temp_lines[i];
            let end = if i + 1 < temp_lines.len() {
                temp_lines[i + 1].0
            } else {
                start + 5000.0 // 默认5秒
            };
            
            lyric.lines.push(LyricLine::new(*start, text.clone())
                .with_duration(end - start));
        }
        
        Ok(lyric)
    }
    
    /// 解析时间标签
    fn parse_time_tag(&self, key: &str, value: &str) -> Option<f64> {
        // [mm:ss.xx] 格式
        if let Ok(mins) = key.parse::<f64>() {
            if let Ok(secs) = value[..2.min(value.len())].parse::<f64>() {
                let ms = if value.len() > 3 {
                    value[3..].parse::<f64>().unwrap_or(0.0)
                } else {
                    0.0
                } * if value.len() > 4 { 10.0 } else { 1.0 };
                return Some((mins * 60.0 + secs) * 1000.0 + ms);
            }
        }
        None
    }
    
    /// 提取歌词文本（移除时间标签）
    fn extract_text(&self, line: &str) -> String {
        self.time_regex.replace_all(line, "").trim().to_string()
    }
    
    /// 使用指定格式解析
    pub fn parse_with_format(&self, content: &str, format: LyricFormat) -> Result<ParsedLyric> {
        let format = match format {
            LyricFormat::Auto => LyricFormat::detect(content),
            _ => format,
        };
        
        match format {
            LyricFormat::Lrc => self.parse_lrc(content),
            LyricFormat::Krc => self.parse_krc(content),
            LyricFormat::Yrc => self.parse_yrc(content),
            LyricFormat::Qrc => self.parse_qrc(content),
            _ => Err(Error::LyricParseFailed(format!("Unsupported format: {:?}", format))),
        }
    }
    
    /// 解析 KRC 格式（酷狗加密）
    fn parse_krc(&self, content: &str) -> Result<ParsedLyric> {
        // KRC 通常是加密的，需要先用 crypto 模块解密
        // 这里假设已经解密为类 LRC 格式
        self.parse_lrc(content)
    }
    
    /// 解析 YRC 格式（网易云逐字）
    fn parse_yrc(&self, content: &str) -> Result<ParsedLyric> {
        // YRC 格式: [timestamp,duration]word<start,duration>word
        let mut lyric = ParsedLyric::empty();
        
        for line in content.lines() {
            if let Some((time_part, text_part)) = line.split_once(']') {
                let time_part = time_part.trim_start_matches('[');
                let parts: Vec<&str> = time_part.split(',').collect();
                
                if parts.len() >= 2 {
                    let start_time: f64 = parts[0].parse().unwrap_or(0.0);
                    let duration: f64 = parts[1].parse().unwrap_or(0.0);
                    
                    // 解析逐字时间
                    let mut words = Vec::new();
                    let mut text = String::new();
                    let mut char_offset = 0;
                    
                    for cap in self.word_time_regex.captures_iter(text_part) {
                        let w_start: f64 = cap[1].parse().unwrap_or(0.0);
                        let w_duration: f64 = cap[2].parse().unwrap_or(0.0);
                        let w_chars: usize = cap[3].parse().unwrap_or(1);
                        
                        let word_text = "..."; // 需要提取实际文本
                        
                        words.push(WordTiming::new(
                            char_offset,
                            w_chars,
                            w_start,
                            w_duration,
                            word_text,
                        ));
                        
                        char_offset += w_chars;
                    }
                    
                    text = self.word_time_regex.replace_all(text_part, "").to_string();
                    
                    // 在移动前检查是否有逐字时间
                    let has_word_timing = !words.is_empty();
                    
                    lyric.lines.push(LyricLine::new(start_time, text)
                        .with_duration(duration)
                        .with_words(words));
                    
                    lyric.has_word_timing = has_word_timing;
                }
            }
        }
        
        Ok(lyric)
    }
    
    /// 解析 QRC 格式（QQ音乐）
    fn parse_qrc(&self, content: &str) -> Result<ParsedLyric> {
        // QRC 是 XML 格式
        // 简化处理，提取时间戳和文本
        self.parse_lrc(content)
    }
    
    /// 合并翻译
    fn merge_translation(&self, main: &mut ParsedLyric, translation: &ParsedLyric) {
        for trans_line in &translation.lines {
            // 查找时间最接近的行
            if let Some(main_line) = main.lines.iter_mut()
                .min_by(|a, b| {
                    let da = (a.start_time - trans_line.start_time).abs();
                    let db = (b.start_time - trans_line.start_time).abs();
                    da.partial_cmp(&db).unwrap()
                }) {
                if (main_line.start_time - trans_line.start_time).abs() < 1000.0 {
                    main_line.translation = Some(trans_line.text.clone());
                }
            }
        }
    }
    
    /// 合并罗马音
    fn merge_romaji(&self, main: &mut ParsedLyric, romaji: &ParsedLyric) {
        for roma_line in &romaji.lines {
            if let Some(main_line) = main.lines.iter_mut()
                .min_by(|a, b| {
                    let da = (a.start_time - roma_line.start_time).abs();
                    let db = (b.start_time - roma_line.start_time).abs();
                    da.partial_cmp(&db).unwrap()
                }) {
                if (main_line.start_time - roma_line.start_time).abs() < 1000.0 {
                    main_line.romaji = Some(roma_line.text.clone());
                }
            }
        }
    }
    
    /// 解析逐字时间信息
    fn parse_word_timing(&self, lyric: &mut ParsedLyric, lxlyric: &str) -> Result<()> {
        // LX 格式逐字歌词
        // 格式: [timestamp]word<start,duration>word
        lyric.has_word_timing = true;
        Ok(())
    }
}

impl Default for LyricParser {
    fn default() -> Self {
        Self::new()
    }
}
