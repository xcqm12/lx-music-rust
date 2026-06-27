//! Lyric Processing Module
//! 
//! Handles LRC lyric parsing, time synchronization,
//! translation merging, and display logic.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use regex::Regex;
use once_cell::sync::Lazy;

/// Pre-compiled LRC timestamp regex pattern
static LRC_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[(\d{1,2}):(\d{2})\.(\d{2,4})\](.*)").unwrap()
});

/// Lyric line structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricLine {
    pub time: f64,
    pub text: String,
    pub translation: Option<String>,
}

impl LyricLine {
    /// Create a new lyric line
    pub fn new(time: f64, text: &str) -> Self {
        LyricLine {
            time,
            text: text.to_string(),
            translation: None,
        }
    }

    /// Create with translation
    pub fn with_translation(time: f64, text: &str, translation: &str) -> Self {
        LyricLine {
            time,
            text: text.to_string(),
            translation: Some(translation.to_string()),
        }
    }
}

/// Lyric data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricData {
    pub lines: Vec<LyricLine>,
    #[serde(rename = "translationLines")]
    pub translation_lines: Vec<LyricLine>,
    #[serde(rename = "rawLyric")]
    pub raw_lyric: String,
    #[serde(rename = "rawTranslation")]
    pub raw_translation: String,
    #[serde(rename = "isShowTranslation")]
    pub is_show_translation: bool,
    #[serde(rename = "isShowRoma")]
    pub is_show_roma: bool,
    #[serde(rename = "playbackRate")]
    pub playback_rate: f32,
}

impl Default for LyricData {
    fn default() -> Self {
        LyricData {
            lines: Vec::new(),
            translation_lines: Vec::new(),
            raw_lyric: String::new(),
            raw_translation: String::new(),
            is_show_translation: false,
            is_show_roma: false,
            playback_rate: 1.0,
        }
    }
}

/// Lyric result for parsed content
#[derive(Debug, Clone)]
pub struct LyricResult {
    pub lines: Vec<LyricLine>,
    pub translation_lines: Vec<LyricLine>,
    pub raw_lyric: String,
    pub raw_translation: String,
}

/// Lyric engine
pub struct LyricEngine {
    data: Arc<RwLock<LyricData>>,
}

impl LyricEngine {
    /// Create a new lyric engine
    pub fn new() -> Self {
        LyricEngine {
            data: Arc::new(RwLock::new(LyricData::default())),
        }
    }

    /// Parse LRC format lyric text
    pub fn parse_lrc(&self, lrc_content: &str) -> Vec<LyricLine> {
        let line_count = lrc_content.lines().count();
        let mut lines = Vec::with_capacity(line_count);

        for line in lrc_content.lines() {
            let trimmed = line.trim();
            
            if let Some(cap) = LRC_PATTERN.captures(trimmed) {
                let minutes: f64 = cap[1].parse().unwrap_or(0.0);
                let seconds: f64 = cap[2].parse().unwrap_or(0.0);
                let milliseconds: f64 = cap[3].parse().unwrap_or(0.0);
                
                let time = minutes * 60.0 + seconds + milliseconds / 1000.0;
                let text = cap[4].trim().to_string();
                
                if !text.is_empty() {
                    lines.push(LyricLine::new(time, &text));
                }
            }
        }

        lines.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        lines
    }

    /// Parse LRC file with translation support
    pub fn parse_lrc_file(&self, lrc_content: &str) -> LyricResult {
        let mut lines = Vec::new();
        let mut translation_lines = Vec::new();
        let mut raw_lyric = String::new();
        let mut raw_translation = String::new();

        for line in lrc_content.lines() {
            let trimmed = line.trim();
            
            if let Some(cap) = LRC_PATTERN.captures(trimmed) {
                let minutes: f64 = cap[1].parse().unwrap_or(0.0);
                let seconds: f64 = cap[2].parse().unwrap_or(0.0);
                let milliseconds: f64 = cap[3].parse().unwrap_or(0.0);
                
                let time = minutes * 60.0 + seconds + milliseconds / 1000.0;
                let text = cap[4].trim().to_string();
                
                if text.is_empty() {
                    continue;
                }

                // Check for translation tag [t:] or extended tags
                if trimmed.starts_with("[t:") {
                    raw_translation.push_str(line);
                    raw_translation.push('\n');
                    translation_lines.push(LyricLine::new(time, &text));
                } else {
                    raw_lyric.push_str(line);
                    raw_lyric.push('\n');
                    lines.push(LyricLine::new(time, &text));
                }
            }
        }

        lines.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        translation_lines.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

        LyricResult {
            lines,
            translation_lines,
            raw_lyric,
            raw_translation,
        }
    }

    /// Merge translations with original lyrics
    pub fn merge_translation(&self, lyrics: &[LyricLine], translations: &[LyricLine]) -> Vec<LyricLine> {
        let mut result = Vec::new();
        let mut trans_index = 0;

        for lyric in lyrics {
            let mut line = lyric.clone();
            
            while trans_index < translations.len() {
                let trans = &translations[trans_index];
                let time_diff = (trans.time - lyric.time).abs();
                
                if time_diff < 0.5 {
                    line.translation = Some(trans.text.clone());
                    trans_index += 1;
                    break;
                } else if trans.time < lyric.time - 0.5 {
                    trans_index += 1;
                } else {
                    break;
                }
            }
            
            result.push(line);
        }

        result
    }

    /// Set lyrics with optional translation
    pub fn set_lyric(&self, lyric: &str, translation: &str) {
        let mut data = self.data.write().unwrap();
        
        let parsed = self.parse_lrc_file(lyric);
        let trans_parsed = if !translation.is_empty() {
            self.parse_lrc_file(translation)
        } else {
            LyricResult {
                lines: Vec::new(),
                translation_lines: Vec::new(),
                raw_lyric: String::new(),
                raw_translation: String::new(),
            }
        };

        let merged = self.merge_translation(&parsed.lines, &trans_parsed.lines);

        data.lines = merged;
        data.translation_lines = trans_parsed.lines;
        data.raw_lyric = lyric.to_string();
        data.raw_translation = translation.to_string();
    }

    /// Set raw lyric only
    pub fn set_raw_lyric(&self, lyric: &str) {
        let mut data = self.data.write().unwrap();
        let parsed = self.parse_lrc_file(lyric);
        data.lines = parsed.lines;
        data.raw_lyric = lyric.to_string();
    }

    /// Get current lyric line at given time
    pub fn get_current_line(&self, time_ms: u64) -> Option<LyricLine> {
        let data = self.data.read().unwrap();
        let time = time_ms as f64 / 1000.0 / (data.playback_rate as f64);

        find_line_index(&data.lines, time)
            .map(|i| data.lines[i].clone())
    }

    /// Get current lyric line index at given time
    pub fn get_line_index(&self, time_ms: u64) -> i32 {
        let data = self.data.read().unwrap();
        let time = time_ms as f64 / 1000.0 / (data.playback_rate as f64);

        find_line_index(&data.lines, time)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    /// Get all lyric lines
    pub fn get_lines(&self) -> Vec<LyricLine> {
        self.data.read().unwrap().lines.clone()
    }

    /// Get lines with range around center index
    pub fn get_lines_with_range(&self, center_index: i32, count: usize) -> Vec<LyricLine> {
        let data = self.data.read().unwrap();
        let start = std::cmp::max(0, center_index - (count as i32 / 2)) as usize;
        let end = std::cmp::min(data.lines.len(), start + count);
        
        if start < end {
            data.lines[start..end].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Set playback rate
    pub fn set_playback_rate(&self, rate: f32) {
        let mut data = self.data.write().unwrap();
        data.playback_rate = rate;
    }

    /// Toggle translation display
    pub fn toggle_translation(&self, show: bool) {
        let mut data = self.data.write().unwrap();
        data.is_show_translation = show;
    }

    /// Check if translation is shown
    pub fn is_show_translation(&self) -> bool {
        self.data.read().unwrap().is_show_translation
    }

    /// Clear all lyrics
    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        *data = LyricData::default();
    }

    /// Get lyric time at index
    pub fn get_lyric_time(&self, index: usize) -> Option<f64> {
        self.data.read().unwrap().lines.get(index).map(|line| line.time)
    }

    /// Get lines as JSON
    pub fn get_lines_json(&self) -> String {
        let lines = self.get_lines();
        serde_json::to_string(&lines).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get current line as JSON
    pub fn get_current_line_json(&self, time_ms: u64) -> String {
        let line = self.get_current_line(time_ms);
        serde_json::to_string(&line).unwrap_or_else(|_| "null".to_string())
    }
}

/// Find the lyric line index for a given time using binary search.
/// Returns the index of the line that should be displayed at the given time.
fn find_line_index(lines: &[LyricLine], time: f64) -> Option<usize> {
    if lines.is_empty() {
        return None;
    }
    // Binary search for the first line with time > target
    let idx = lines.partition_point(|line| line.time <= time);
    if idx == 0 {
        // Before first line, return None
        None
    } else {
        Some(idx - 1)
    }
}

impl Default for LyricEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // LyricLine tests
    // ========================================================================
    #[test]
    fn test_lyric_line_new() {
        let line = LyricLine::new(10.5, "hello");
        assert_eq!(line.time, 10.5);
        assert_eq!(line.text, "hello");
        assert!(line.translation.is_none());
    }

    #[test]
    fn test_lyric_line_with_translation() {
        let line = LyricLine::with_translation(5.0, "你好", "Hello");
        assert_eq!(line.time, 5.0);
        assert_eq!(line.text, "你好");
        assert_eq!(line.translation, Some("Hello".to_string()));
    }

    // ========================================================================
    // LyricData tests
    // ========================================================================
    #[test]
    fn test_lyric_data_default() {
        let data = LyricData::default();
        assert!(data.lines.is_empty());
        assert!(data.translation_lines.is_empty());
        assert!(data.raw_lyric.is_empty());
        assert!(!data.is_show_translation);
        assert!(!data.is_show_roma);
        assert_eq!(data.playback_rate, 1.0);
    }

    // ========================================================================
    // LRC parsing tests
    // ========================================================================
    #[test]
    fn test_parse_lrc_basic() {
        let engine = LyricEngine::new();
        let lrc = "[00:01.500]Hello\n[00:03.000]World\n[00:05.250]Test";
        let lines = engine.parse_lrc(lrc);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].time, 1.5);
        assert_eq!(lines[0].text, "Hello");
        assert_eq!(lines[1].time, 3.0);
        assert_eq!(lines[1].text, "World");
        assert_eq!(lines[2].time, 5.25);
        assert_eq!(lines[2].text, "Test");
    }

    #[test]
    fn test_parse_lrc_empty_lines() {
        let engine = LyricEngine::new();
        let lrc = "[00:01.00]Hello\n[00:02.00]\n[00:03.00]World";
        let lines = engine.parse_lrc(lrc);
        assert_eq!(lines.len(), 2); // empty text line is skipped
    }

    #[test]
    fn test_parse_lrc_sorted() {
        let engine = LyricEngine::new();
        let lrc = "[00:03.00]C\n[00:01.00]A\n[00:02.00]B";
        let lines = engine.parse_lrc(lrc);
        assert_eq!(lines[0].text, "A");
        assert_eq!(lines[1].text, "B");
        assert_eq!(lines[2].text, "C");
    }

    #[test]
    fn test_parse_lrc_invalid_format() {
        let engine = LyricEngine::new();
        let lrc = "not a valid lrc line\n[ti:Title]\n[ar:Artist]";
        let lines = engine.parse_lrc(lrc);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_parse_lrc_file() {
        let engine = LyricEngine::new();
        let lrc = "[00:01.00]Hello\n[00:02.00]World";
        let result = engine.parse_lrc_file(lrc);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.raw_lyric.lines().count(), 2);
    }

    // ========================================================================
    // Translation merging tests
    // ========================================================================
    #[test]
    fn test_merge_translation() {
        let engine = LyricEngine::new();
        let lyrics = vec![
            LyricLine::new(1.0, "Hello"),
            LyricLine::new(2.0, "World"),
        ];
        let translations = vec![
            LyricLine::new(1.0, "你好"),
            LyricLine::new(2.0, "世界"),
        ];
        let merged = engine.merge_translation(&lyrics, &translations);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].translation, Some("你好".to_string()));
        assert_eq!(merged[1].translation, Some("世界".to_string()));
    }

    #[test]
    fn test_merge_translation_near_match() {
        let engine = LyricEngine::new();
        let lyrics = vec![
            LyricLine::new(1.0, "Hello"),
        ];
        let translations = vec![
            LyricLine::new(1.3, "你好"), // within 0.5s tolerance
        ];
        let merged = engine.merge_translation(&lyrics, &translations);
        assert_eq!(merged[0].translation, Some("你好".to_string()));
    }

    #[test]
    fn test_merge_translation_far_match() {
        let engine = LyricEngine::new();
        let lyrics = vec![
            LyricLine::new(1.0, "Hello"),
        ];
        let translations = vec![
            LyricLine::new(2.0, "你好"), // >0.5s gap
        ];
        let merged = engine.merge_translation(&lyrics, &translations);
        assert_eq!(merged[0].translation, None);
    }

    #[test]
    fn test_merge_translation_empty() {
        let engine = LyricEngine::new();
        let lyrics = vec![LyricLine::new(1.0, "Hello")];
        let merged = engine.merge_translation(&lyrics, &[]);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].translation.is_none());
    }

    // ========================================================================
    // Set/Get lyric tests
    // ========================================================================
    #[test]
    fn test_set_lyric() {
        let engine = LyricEngine::new();
        engine.set_lyric("[00:01.00]Hello\n[00:02.00]World", "");
        let lines = engine.get_lines();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_set_raw_lyric() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]Hello");
        let lines = engine.get_lines();
        assert_eq!(lines.len(), 1);
    }

    // ========================================================================
    // Current line lookup tests
    // ========================================================================
    #[test]
    fn test_get_current_line() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A\n[00:02.00]B\n[00:03.00]C");
        let line = engine.get_current_line(1500); // 1.5s
        assert!(line.is_some());
        assert_eq!(line.unwrap().text, "A");
    }

    #[test]
    fn test_get_current_line_before_first() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A\n[00:02.00]B");
        let line = engine.get_current_line(500); // 0.5s
        assert!(line.is_none());
    }

    #[test]
    fn test_get_current_line_after_last() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A\n[00:02.00]B");
        // After last line time, returns the last line (f64::MAX as next_time)
        let line = engine.get_current_line(10000); // 10s
        assert!(line.is_some());
        assert_eq!(line.unwrap().text, "B");
    }

    #[test]
    fn test_get_line_index() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A\n[00:02.00]B\n[00:03.00]C");
        assert_eq!(engine.get_line_index(1500), 0); // 1.5s → line 0
        assert_eq!(engine.get_line_index(2500), 1); // 2.5s → line 1
    }

    #[test]
    fn test_get_line_index_not_found() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A");
        assert_eq!(engine.get_line_index(0), -1);
    }

    // ========================================================================
    // Lines with range tests
    // ========================================================================
    #[test]
    fn test_get_lines_with_range() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A\n[00:02.00]B\n[00:03.00]C\n[00:04.00]D\n[00:05.00]E");
        let lines = engine.get_lines_with_range(2, 3);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "B");
        assert_eq!(lines[2].text, "D");
    }

    #[test]
    fn test_get_lines_with_range_near_start() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A\n[00:02.00]B\n[00:03.00]C");
        let lines = engine.get_lines_with_range(0, 3);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_get_lines_with_range_empty() {
        let engine = LyricEngine::new();
        let lines = engine.get_lines_with_range(0, 5);
        assert!(lines.is_empty());
    }

    // ========================================================================
    // Playback rate tests
    // ========================================================================
    #[test]
    fn test_set_playback_rate() {
        let engine = LyricEngine::new();
        engine.set_playback_rate(1.5);
        // Verify get_line_index accounts for playback rate
        engine.set_raw_lyric("[00:01.00]A\n[00:02.00]B");
        // At 1.5x rate, 1500ms real time = 1000ms lyric time
        assert_eq!(engine.get_line_index(1500), 0);
    }

    // ========================================================================
    // Toggle translation tests
    // ========================================================================
    #[test]
    fn test_toggle_translation() {
        let engine = LyricEngine::new();
        assert!(!engine.is_show_translation());
        engine.toggle_translation(true);
        assert!(engine.is_show_translation());
        engine.toggle_translation(false);
        assert!(!engine.is_show_translation());
    }

    // ========================================================================
    // Clear tests
    // ========================================================================
    #[test]
    fn test_clear() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.00]A");
        engine.clear();
        assert!(engine.get_lines().is_empty());
    }

    // ========================================================================
    // Get lyric time tests
    // ========================================================================
    #[test]
    fn test_get_lyric_time() {
        let engine = LyricEngine::new();
        engine.set_raw_lyric("[00:01.500]A\n[00:03.000]B");
        assert_eq!(engine.get_lyric_time(0), Some(1.5));
        assert_eq!(engine.get_lyric_time(1), Some(3.0));
        assert_eq!(engine.get_lyric_time(99), None);
    }

    // ========================================================================
    // JSON output tests
    // ========================================================================
    #[test]
    fn test_get_lines_json() {
        let engine = LyricEngine::new();
        let json = engine.get_lines_json();
        assert_eq!(json, "[]");
        engine.set_raw_lyric("[00:01.00]A");
        let json = engine.get_lines_json();
        assert!(json.contains("A"));
    }

    #[test]
    fn test_get_current_line_json() {
        let engine = LyricEngine::new();
        let json = engine.get_current_line_json(0);
        assert_eq!(json, "null");
        engine.set_raw_lyric("[00:01.00]A");
        let json = engine.get_current_line_json(1500);
        assert!(json.contains("A"));
    }
}