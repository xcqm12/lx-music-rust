use crate::types::ParsedLyric;
use crate::{LyricEvent, LyricManager, LyricSync};
use std::sync::Arc;
use tokio::sync::mpsc;

/// 高级歌词管理器
pub struct AdvancedLyricManager {
    manager: LyricManager,
    sync: LyricSync,
    event_receiver: mpsc::Receiver<LyricEvent>,
}

impl AdvancedLyricManager {
    pub fn new() -> (Self, mpsc::Receiver<LyricEvent>) {
        let (event_sender, event_receiver) = mpsc::channel(100);
        
        let manager = LyricManager::new(event_sender.clone());
        let sync = LyricSync::new(event_sender);
        
        (Self {
            manager,
            sync,
            event_receiver,
        }, event_receiver)
    }
    
    /// 获取内部管理器
    pub fn manager(&self) -> &LyricManager {
        &self.manager
    }
    
    /// 获取同步器
    pub fn sync(&self) -> &LyricSync {
        &self.sync
    }
}

/// 歌词缓存管理器
pub struct LyricCache {
    cache: dashmap::DashMap<String, CachedLyric>,
    max_size: usize,
}

#[derive(Clone)]
struct CachedLyric {
    lyric: ParsedLyric,
    timestamp: std::time::Instant,
}

impl LyricCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: dashmap::DashMap::new(),
            max_size,
        }
    }
    
    /// 获取缓存的歌词
    pub fn get(&self, key: &str) -> Option<ParsedLyric> {
        self.cache.get(key).map(|entry| {
            entry.value().lyric.clone()
        })
    }
    
    /// 缓存歌词
    pub fn put(&self, key: String, lyric: ParsedLyric) {
        // 如果达到最大容量，清理旧的
        if self.cache.len() >= self.max_size {
            self.cleanup();
        }
        
        self.cache.insert(key, CachedLyric {
            lyric,
            timestamp: std::time::Instant::now(),
        });
    }
    
    /// 清理过期缓存
    fn cleanup(&self) {
        let mut entries: Vec<_> = self.cache
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().timestamp))
            .collect();
        
        // 按时间排序，删除最旧的
        entries.sort_by(|a, b| a.1.cmp(&b.1));
        
        let to_remove = entries.len() / 4; // 删除 25%
        for (key, _) in entries.iter().take(to_remove) {
            self.cache.remove(key);
        }
    }
    
    /// 清空缓存
    pub fn clear(&self) {
        self.cache.clear();
    }
    
    /// 获取缓存大小
    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

/// 歌词偏好设置
#[derive(Debug, Clone)]
pub struct LyricPreferences {
    /// 显示翻译
    pub show_translation: bool,
    /// 显示罗马音
    pub show_romaji: bool,
    /// 歌词偏移（毫秒）
    pub offset: f64,
    /// 字体大小
    pub font_size: f32,
    /// 高亮颜色
    pub highlight_color: String,
    /// 普通颜色
    pub normal_color: String,
}

impl Default for LyricPreferences {
    fn default() -> Self {
        Self {
            show_translation: true,
            show_romaji: false,
            offset: 0.0,
            font_size: 16.0,
            highlight_color: "#FF6B6B".to_string(),
            normal_color: "#CCCCCC".to_string(),
        }
    }
}
