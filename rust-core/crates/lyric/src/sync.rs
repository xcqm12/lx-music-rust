use crate::types::{LyricLine, ParsedLyric};
use crate::LyricEvent;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// 歌词同步器
pub struct LyricSync {
    current_time: Arc<RwLock<f64>>,
    current_line: Arc<RwLock<usize>>,
    current_word: Arc<RwLock<Option<(usize, usize)>>>, // (line, word)
    event_sender: mpsc::Sender<LyricEvent>,
    offset: Arc<RwLock<f64>>,
}

impl LyricSync {
    pub fn new(event_sender: mpsc::Sender<LyricEvent>) -> Self {
        Self {
            current_time: Arc::new(RwLock::new(0.0)),
            current_line: Arc::new(RwLock::new(0)),
            current_word: Arc::new(RwLock::new(None)),
            event_sender,
            offset: Arc::new(RwLock::new(0.0)),
        }
    }
    
    /// 更新时间
    pub async fn update_time(&self, time_ms: f64, lyric: &ParsedLyric) {
        let offset = *self.offset.read().await;
        let adjusted_time = time_ms + offset;
        
        *self.current_time.write().await = time_ms;
        
        // 更新当前行
        let new_line = lyric.find_current_line(adjusted_time);
        let old_line = *self.current_line.read().await;
        
        if new_line != old_line {
            *self.current_line.write().await = new_line;
            
            // 发送行变化事件
            if let Some(line) = lyric.lines.get(new_line) {
                let _ = self.event_sender
                    .send(LyricEvent::LineChanged(new_line, line.clone()))
                    .await;
            }
        }
        
        // 更新当前字（逐字歌词）
        if lyric.has_word_timing {
            if let Some(line) = lyric.lines.get(new_line) {
                if let Some(word_idx) = line.find_current_word(adjusted_time) {
                    let old_word = *self.current_word.read().await;
                    let new_word = Some((new_line, word_idx));
                    
                    if old_word != new_word {
                        *self.current_word.write().await = new_word;
                        
                        if let Some((line_idx, word_idx)) = new_word {
                            let _ = self.event_sender
                                .send(LyricEvent::WordChanged(line_idx, word_idx, adjusted_time))
                                .await;
                        }
                    }
                }
            }
        }
    }
    
    /// 获取当前行索引
    pub async fn current_line(&self) -> usize {
        *self.current_line.read().await
    }
    
    /// 获取当前时间
    pub async fn current_time(&self) -> f64 {
        *self.current_time.read().await
    }
    
    /// 获取当前字索引
    pub async fn current_word(&self) -> Option<(usize, usize)> {
        *self.current_word.read().await
    }
    
    /// 设置偏移量
    pub async fn set_offset(&self, offset: f64) {
        *self.offset.write().await = offset;
    }
    
    /// 获取偏移量
    pub async fn offset(&self) -> f64 {
        *self.offset.read().await
    }
    
    /// 调整偏移量（微调）
    pub async fn adjust_offset(&self, delta: f64) {
        *self.offset.write().await += delta;
    }
    
    /// 重置状态
    pub async fn reset(&self) {
        *self.current_time.write().await = 0.0;
        *self.current_line.write().await = 0;
        *self.current_word.write().await = None;
    }
}

/// 歌词滚动控制器
pub struct LyricScroller {
    /// 当前视图起始行
    view_start: Arc<RwLock<usize>>,
    /// 视图大小（可见行数）
    view_size: Arc<RwLock<usize>>,
    /// 高亮行偏移（高亮行在视图中的位置）
    highlight_offset: Arc<RwLock<usize>>,
}

impl LyricScroller {
    pub fn new(view_size: usize, highlight_offset: usize) -> Self {
        Self {
            view_start: Arc::new(RwLock::new(0)),
            view_size: Arc::new(RwLock::new(view_size)),
            highlight_offset: Arc::new(RwLock::new(highlight_offset)),
        }
    }
    
    /// 更新视图位置
    pub async fn update(&self, current_line: usize, total_lines: usize) {
        let view_size = *self.view_size.read().await;
        let highlight_offset = *self.highlight_offset.read().await;
        
        let mut view_start = *self.view_start.read().await;
        
        // 计算目标视图起始位置
        let target_start = if current_line >= highlight_offset {
            current_line - highlight_offset
        } else {
            0
        };
        
        // 确保不超出范围
        let max_start = if total_lines > view_size {
            total_lines - view_size
        } else {
            0
        };
        
        view_start = target_start.min(max_start);
        
        *self.view_start.write().await = view_start;
    }
    
    /// 获取视图起始行
    pub async fn view_start(&self) -> usize {
        *self.view_start.read().await
    }
    
    /// 获取视图结束行
    pub async fn view_end(&self) -> usize {
        let start = *self.view_start.read().await;
        let size = *self.view_size.read().await;
        start + size
    }
    
    /// 获取可见行范围
    pub async fn visible_range(&self) -> (usize, usize) {
        let start = self.view_start().await;
        let end = self.view_end().await;
        (start, end)
    }
    
    /// 设置视图大小
    pub async fn set_view_size(&self, size: usize) {
        *self.view_size.write().await = size;
    }
    
    /// 重置视图
    pub async fn reset(&self) {
        *self.view_start.write().await = 0;
    }
}
