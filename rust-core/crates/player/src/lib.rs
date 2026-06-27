pub mod audio_engine;
pub mod decoder;
pub mod playlist;
pub mod events;

use common::{MusicInfo, PlayState, PlayMode, PlayProgress, PlayerConfig, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// 播放器核心
pub struct Player {
    config: Arc<RwLock<PlayerConfig>>,
    state: Arc<RwLock<PlayState>>,
    progress: Arc<RwLock<PlayProgress>>,
    playlist: playlist::PlaylistManager,
    event_sender: mpsc::Sender<PlayerEvent>,
    audio_engine: Arc<RwLock<audio_engine::AudioEngine>>,
}

/// 播放器事件
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlayState),
    ProgressChanged(PlayProgress),
    TrackChanged(MusicInfo),
    Buffering(f32), // 缓冲进度 0.0-1.0
    Error(String),
    Completed,
}

impl Player {
    pub async fn new(config: PlayerConfig) -> Result<(Self, mpsc::Receiver<PlayerEvent>)> {
        let (event_sender, event_receiver) = mpsc::channel(100);
        
        let player = Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(PlayState::Idle)),
            progress: Arc::new(RwLock::new(PlayProgress {
                position: 0.0,
                duration: 0.0,
                buffered: 0.0,
            })),
            playlist: playlist::PlaylistManager::new(),
            event_sender,
            audio_engine: Arc::new(RwLock::new(audio_engine::AudioEngine::new()?)),
        };
        
        Ok((player, event_receiver))
    }
    
    /// 播放
    pub async fn play(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = PlayState::Playing;
        self.event_sender.send(PlayerEvent::StateChanged(PlayState::Playing)).await.ok();
        Ok(())
    }
    
    /// 暂停
    pub async fn pause(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = PlayState::Paused;
        self.event_sender.send(PlayerEvent::StateChanged(PlayState::Paused)).await.ok();
        Ok(())
    }
    
    /// 停止
    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = PlayState::Stopped;
        self.event_sender.send(PlayerEvent::StateChanged(PlayState::Stopped)).await.ok();
        Ok(())
    }
    
    /// 跳转到指定位置
    pub async fn seek(&self, position: f64) -> Result<()> {
        let mut progress = self.progress.write().await;
        progress.position = position;
        Ok(())
    }
    
    /// 设置音量
    pub async fn set_volume(&self, volume: f32) -> Result<()> {
        let mut config = self.config.write().await;
        config.volume = volume.clamp(0.0, 1.0);
        Ok(())
    }
    
    /// 获取当前音量
    pub async fn get_volume(&self) -> f32 {
        self.config.read().await.volume
    }
    
    /// 设置播放模式
    pub async fn set_play_mode(&self, mode: PlayMode) -> Result<()> {
        let mut config = self.config.write().await;
        config.play_mode = mode;
        Ok(())
    }
    
    /// 获取播放模式
    pub async fn get_play_mode(&self) -> PlayMode {
        self.config.read().await.play_mode
    }
    
    /// 获取播放状态
    pub async fn get_state(&self) -> PlayState {
        *self.state.read().await
    }
    
    /// 获取播放进度
    pub async fn get_progress(&self) -> PlayProgress {
        self.progress.read().await.clone()
    }
    
    /// 获取当前曲目
    pub async fn get_current_track(&self) -> Option<MusicInfo> {
        self.playlist.current()
    }
    
    /// 播放指定歌曲
    pub async fn play_track(&self, music_info: MusicInfo) -> Result<()> {
        self.playlist.set_current(music_info.clone());
        self.event_sender.send(PlayerEvent::TrackChanged(music_info)).await.ok();
        self.play().await
    }
    
    /// 下一首
    pub async fn next(&self) -> Result<()> {
        if let Some(next) = self.playlist.next() {
            self.play_track(next).await
        } else {
            Ok(())
        }
    }
    
    /// 上一首
    pub async fn previous(&self) -> Result<()> {
        if let Some(prev) = self.playlist.previous() {
            self.play_track(prev).await
        } else {
            Ok(())
        }
    }
    
    /// 添加到播放列表
    pub async fn add_to_playlist(&self, music_info: MusicInfo) {
        self.playlist.add(music_info);
    }
    
    /// 清空播放列表
    pub async fn clear_playlist(&self) {
        self.playlist.clear();
    }
    
    /// 设置播放列表
    pub async fn set_playlist(&self, tracks: Vec<MusicInfo>) {
        self.playlist.set_list(tracks);
    }
}
