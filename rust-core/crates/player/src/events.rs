/// 播放器事件处理
use common::{MusicInfo, PlayState, PlayProgress};

/// 播放器事件回调 trait
pub trait PlayerEventHandler: Send + Sync {
    /// 播放状态变化
    fn on_state_changed(&self, state: PlayState);
    
    /// 播放进度变化
    fn on_progress_changed(&self, progress: PlayProgress);
    
    /// 歌曲切换
    fn on_track_changed(&self, music_info: MusicInfo);
    
    /// 缓冲进度
    fn on_buffering(&self, progress: f32);
    
    /// 播放完成
    fn on_completed(&self);
    
    /// 错误
    fn on_error(&self, error: String);
}

/// 播放器事件代理（用于 FFI 回调）
pub struct PlayerEventProxy {
    callback: Box<dyn Fn(PlayerEvent) + Send + Sync>,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlayState),
    ProgressChanged(PlayProgress),
    TrackChanged(MusicInfo),
    Buffering(f32),
    Completed,
    Error(String),
}

impl PlayerEventProxy {
    pub fn new<F>(callback: F) -> Self 
    where 
        F: Fn(PlayerEvent) + Send + Sync + 'static
    {
        Self {
            callback: Box::new(callback),
        }
    }
    
    pub fn emit(&self, event: PlayerEvent) {
        (self.callback)(event);
    }
}

/// 事件聚合器
pub struct EventAggregator {
    handlers: Vec<Box<dyn PlayerEventHandler>>,
}

impl EventAggregator {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    
    pub fn add_handler(&mut self, handler: Box<dyn PlayerEventHandler>) {
        self.handlers.push(handler);
    }
    
    pub fn remove_handler(&mut self, index: usize) {
        if index < self.handlers.len() {
            self.handlers.remove(index);
        }
    }
    
    pub fn emit_state_changed(&self, state: PlayState) {
        for handler in &self.handlers {
            handler.on_state_changed(state.clone());
        }
    }
    
    pub fn emit_progress_changed(&self, progress: PlayProgress) {
        for handler in &self.handlers {
            handler.on_progress_changed(progress.clone());
        }
    }
    
    pub fn emit_track_changed(&self, music_info: MusicInfo) {
        for handler in &self.handlers {
            handler.on_track_changed(music_info.clone());
        }
    }
    
    pub fn emit_buffering(&self, progress: f32) {
        for handler in &self.handlers {
            handler.on_buffering(progress);
        }
    }
    
    pub fn emit_completed(&self) {
        for handler in &self.handlers {
            handler.on_completed();
        }
    }
    
    pub fn emit_error(&self, error: String) {
        for handler in &self.handlers {
            handler.on_error(error.clone());
        }
    }
}
