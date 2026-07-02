pub mod audio_engine;
pub mod decoder;
pub mod playlist;
pub mod events;

use common::{MusicInfo, PlayState, PlayMode, PlayProgress, PlayerConfig, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use audio_engine::AudioEngine;

pub use events::{PlayerEvent, PlayerEventHandler, PlayerEventProxy, EventAggregator};

/// 播放器核心
pub struct Player {
    config: Arc<RwLock<PlayerConfig>>,
    state: Arc<RwLock<PlayState>>,
    progress: Arc<RwLock<PlayProgress>>,
    playlist: playlist::PlaylistManager,
    event_sender: mpsc::Sender<PlayerEvent>,
    audio_engine: Arc<RwLock<AudioEngine>>,
}

impl Player {
    pub async fn new(config: PlayerConfig) -> Result<(Self, mpsc::Receiver<PlayerEvent>)> {
        let (event_sender, event_receiver) = mpsc::channel(100);
        
        let audio_engine = AudioEngine::new()?;
        let audio_engine = Arc::new(RwLock::new(audio_engine));
        
        let player = Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(PlayState::Idle)),
            progress: Arc::new(RwLock::new(PlayProgress {
                position: 0.0,
                duration: 0.0,
                buffered: 0.0,
            })),
            playlist: playlist::PlaylistManager::new(),
            event_sender: event_sender.clone(),
            audio_engine: audio_engine.clone(),
        };
        
        player.start_event_listener(audio_engine.clone(), event_sender.clone());
        
        Ok((player, event_receiver))
    }
    
    fn start_event_listener(
        &self,
        audio_engine: Arc<RwLock<AudioEngine>>,
        event_sender: mpsc::Sender<PlayerEvent>,
    ) {
        let state = self.state.clone();
        let progress = self.progress.clone();
        let playlist = self.playlist.clone();
        
        tokio::spawn(async move {
            let mut engine_guard = audio_engine.write().await;
            let mut engine_receiver = match engine_guard.get_event_receiver() {
                Some(rx) => rx,
                None => {
                    log::warn!("Failed to get audio engine event receiver");
                    return;
                }
            };
            drop(engine_guard);
            
            loop {
                match engine_receiver.recv().await {
                    Some(audio_engine::AudioEvent::PlaybackStarted) => {
                        let mut s = state.write().await;
                        *s = PlayState::Playing;
                        event_sender.send(PlayerEvent::StateChanged(PlayState::Playing)).await.ok();
                    }
                    Some(audio_engine::AudioEvent::PlaybackPaused) => {
                        let mut s = state.write().await;
                        *s = PlayState::Paused;
                        event_sender.send(PlayerEvent::StateChanged(PlayState::Paused)).await.ok();
                    }
                    Some(audio_engine::AudioEvent::PlaybackStopped) => {
                        let mut s = state.write().await;
                        *s = PlayState::Stopped;
                        event_sender.send(PlayerEvent::StateChanged(PlayState::Stopped)).await.ok();
                    }
                    Some(audio_engine::AudioEvent::PositionChanged(pos)) => {
                        let mut p = progress.write().await;
                        p.position = pos;
                        event_sender.send(PlayerEvent::ProgressChanged(p.clone())).await.ok();
                    }
                    Some(audio_engine::AudioEvent::DurationChanged(dur)) => {
                        let mut p = progress.write().await;
                        p.duration = dur;
                        event_sender.send(PlayerEvent::ProgressChanged(p.clone())).await.ok();
                    }
                    Some(audio_engine::AudioEvent::Buffering(buf)) => {
                        let mut p = progress.write().await;
                        p.buffered = buf as f64;
                        event_sender.send(PlayerEvent::Buffering(buf)).await.ok();
                    }
                    Some(audio_engine::AudioEvent::Error(err)) => {
                        let mut s = state.write().await;
                        *s = PlayState::Error;
                        event_sender.send(PlayerEvent::Error(err)).await.ok();
                    }
                    Some(audio_engine::AudioEvent::Completed) => {
                        let s = state.read().await;
                        let current_state = *s;
                        drop(s);
                        
                        if current_state == PlayState::Playing {
                            let next = playlist.next();
                            if let Some(track) = next {
                                let engine = audio_engine.read().await;
                                if let Some(url) = track.quality.iter().next().map(|(_, v)| v.clone()) {
                                    drop(engine);
                                    event_sender.send(PlayerEvent::TrackChanged(track)).await.ok();
                                    let engine = audio_engine.read().await;
                                    let _ = engine.play_url(&url).await;
                                }
                            } else {
                                let mut s = state.write().await;
                                *s = PlayState::Stopped;
                                event_sender.send(PlayerEvent::Completed).await.ok();
                            }
                        }
                    }
                    None => break,
                }
            }
        });
    }
    
    /// 播放
    pub async fn play(&self) -> Result<()> {
        let state = self.state.read().await;
        if *state == PlayState::Paused {
            let engine = self.audio_engine.read().await;
            engine.resume().await?;
        }
        Ok(())
    }
    
    /// 暂停
    pub async fn pause(&self) -> Result<()> {
        let state = self.state.read().await;
        if *state == PlayState::Playing {
            let engine = self.audio_engine.read().await;
            engine.pause().await?;
        }
        Ok(())
    }
    
    /// 停止
    pub async fn stop(&self) -> Result<()> {
        let engine = self.audio_engine.read().await;
        engine.stop().await?;
        Ok(())
    }
    
    /// 跳转到指定位置
    pub async fn seek(&self, position: f64) -> Result<()> {
        let engine = self.audio_engine.read().await;
        engine.seek(position).await?;
        Ok(())
    }
    
    /// 设置音量
    pub async fn set_volume(&self, volume: f32) -> Result<()> {
        let mut config = self.config.write().await;
        config.volume = volume.clamp(0.0, 1.0);
        let engine = self.audio_engine.read().await;
        engine.set_volume(volume).await;
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
        self.playlist.set_mode(mode);
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
        let engine = self.audio_engine.read().await;
        let position = engine.get_position().await;
        let duration = engine.get_duration().await;
        let buffered = engine.get_buffered().await;
        drop(engine);
        
        let mut progress = self.progress.write().await;
        progress.position = position;
        progress.duration = duration;
        progress.buffered = buffered;
        progress.clone()
    }
    
    /// 获取当前曲目
    pub async fn get_current_track(&self) -> Option<MusicInfo> {
        self.playlist.current()
    }
    
    /// 播放指定歌曲
    pub async fn play_track(&self, music_info: MusicInfo) -> Result<()> {
        self.playlist.set_current(music_info.clone());
        self.event_sender.send(PlayerEvent::TrackChanged(music_info.clone())).await.ok();
        
        let url = music_info.quality
            .iter()
            .next()
            .map(|(_, v)| v.clone())
            .ok_or_else(|| common::Error::Player("No audio quality available".to_string()))?;
        
        let engine = self.audio_engine.read().await;
        engine.play_url(&url).await?;
        
        Ok(())
    }
    
    /// 下一首
    pub async fn next(&self) -> Result<()> {
        if let Some(next_track) = self.playlist.next() {
            self.play_track(next_track).await
        } else {
            Ok(())
        }
    }
    
    /// 上一首
    pub async fn previous(&self) -> Result<()> {
        if let Some(prev_track) = self.playlist.previous() {
            self.play_track(prev_track).await
        } else {
            Ok(())
        }
    }
    
    /// 添加到播放列表
    pub async fn add_to_playlist(&self, music_info: MusicInfo) {
        self.playlist.add(music_info);
    }
    
    /// 从播放列表移除
    pub async fn remove_from_playlist(&self, index: usize) -> Option<MusicInfo> {
        self.playlist.remove(index)
    }
    
    /// 清空播放列表
    pub async fn clear_playlist(&self) {
        self.playlist.clear();
        let _ = self.stop().await;
    }
    
    /// 设置播放列表
    pub async fn set_playlist(&self, tracks: Vec<MusicInfo>) {
        self.playlist.set_list(tracks);
    }
    
    /// 获取播放列表
    pub async fn get_playlist(&self) -> Vec<MusicInfo> {
        self.playlist.get_list()
    }
}
