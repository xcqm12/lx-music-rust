use common::{MusicInfo, PlayMode};
use dashmap::DashMap;
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::sync::Arc;

/// 播放列表管理器
pub struct PlaylistManager {
    tracks: Arc<RwLock<Vec<MusicInfo>>>,
    current_index: Arc<RwLock<usize>>,
    history: Arc<RwLock<Vec<usize>>>,
    play_mode: Arc<RwLock<PlayMode>>,
    // 临时播放列表
    temp_tracks: Arc<RwLock<Vec<MusicInfo>>>,
    // 随机播放历史
    played_indices: Arc<DashMap<usize, bool>>,
}

impl PlaylistManager {
    pub fn new() -> Self {
        Self {
            tracks: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
            history: Arc::new(RwLock::new(Vec::new())),
            play_mode: Arc::new(RwLock::new(PlayMode::Order)),
            temp_tracks: Arc::new(RwLock::new(Vec::new())),
            played_indices: Arc::new(DashMap::new()),
        }
    }
    
    /// 获取当前歌曲
    pub fn current(&self) -> Option<MusicInfo> {
        let tracks = self.tracks.read();
        let index = *self.current_index.read();
        tracks.get(index).cloned()
    }
    
    /// 设置当前歌曲
    pub fn set_current(&self, music_info: MusicInfo) {
        let mut tracks = self.tracks.write();
        let mut index = self.current_index.write();
        
        // 查找是否已在列表中
        if let Some(pos) = tracks.iter().position(|t| t.id == music_info.id) {
            *index = pos;
        } else {
            // 添加到列表
            tracks.push(music_info);
            *index = tracks.len() - 1;
        }
        
        // 记录播放历史
        self.history.write().push(*index);
    }
    
    /// 获取下一首
    pub fn next(&self) -> Option<MusicInfo> {
        let mode = *self.play_mode.read();
        let tracks = self.tracks.read();
        
        if tracks.is_empty() {
            return None;
        }
        
        let new_index = match mode {
            PlayMode::Order => {
                let current = *self.current_index.read();
                if current + 1 < tracks.len() {
                    current + 1
                } else {
                    return None; // 列表结束
                }
            }
            PlayMode::Loop => {
                let current = *self.current_index.read();
                (current + 1) % tracks.len()
            }
            PlayMode::Single => {
                *self.current_index.read()
            }
            PlayMode::Random => {
                let mut rng = thread_rng();
                let indices: Vec<usize> = (0..tracks.len()).collect();
                *indices.choose(&mut rng).unwrap_or(&0)
            }
        };
        
        *self.current_index.write() = new_index;
        self.played_indices.insert(new_index, true);
        tracks.get(new_index).cloned()
    }
    
    /// 获取上一首
    pub fn previous(&self) -> Option<MusicInfo> {
        let tracks = self.tracks.read();
        let mut history = self.history.write();
        
        // 从历史记录中恢复
        if history.len() > 1 {
            history.pop(); // 移除当前
            if let Some(&prev_index) = history.last() {
                *self.current_index.write() = prev_index;
                return tracks.get(prev_index).cloned();
            }
        }
        
        // 简单上一首
        let current = *self.current_index.read();
        if current > 0 {
            let new_index = current - 1;
            *self.current_index.write() = new_index;
            tracks.get(new_index).cloned()
        } else {
            None
        }
    }
    
    /// 添加歌曲
    pub fn add(&self, music_info: MusicInfo) {
        self.tracks.write().push(music_info);
    }
    
    /// 批量添加
    pub fn add_batch(&self, items: Vec<MusicInfo>) {
        self.tracks.write().extend(items);
    }
    
    /// 移除歌曲
    pub fn remove(&self, index: usize) -> Option<MusicInfo> {
        let mut tracks = self.tracks.write();
        if index < tracks.len() {
            Some(tracks.remove(index))
        } else {
            None
        }
    }
    
    /// 清空列表
    pub fn clear(&self) {
        self.tracks.write().clear();
        *self.current_index.write() = 0;
        self.history.write().clear();
        self.played_indices.clear();
    }
    
    /// 设置列表
    pub fn set_list(&self, tracks: Vec<MusicInfo>) {
        *self.tracks.write() = tracks;
        *self.current_index.write() = 0;
        self.history.write().clear();
        self.played_indices.clear();
    }
    
    /// 获取列表
    pub fn get_list(&self) -> Vec<MusicInfo> {
        self.tracks.read().clone()
    }
    
    /// 设置播放模式
    pub fn set_mode(&self, mode: PlayMode) {
        *self.play_mode.write() = mode;
    }
    
    /// 获取播放模式
    pub fn get_mode(&self) -> PlayMode {
        *self.play_mode.read()
    }
    
    /// 移动到指定索引
    pub fn move_to(&self, index: usize) -> Option<MusicInfo> {
        let tracks = self.tracks.read();
        if index < tracks.len() {
            *self.current_index.write() = index;
            self.history.write().push(index);
            tracks.get(index).cloned()
        } else {
            None
        }
    }
    
    /// 获取列表长度
    pub fn len(&self) -> usize {
        self.tracks.read().len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.tracks.read().is_empty()
    }
}
