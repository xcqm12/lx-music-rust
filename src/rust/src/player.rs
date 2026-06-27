//! Playback Engine Module
//! 
//! Handles music playback state management, playlist control,
//! and playback mode selection.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use crossbeam_channel::{Sender, unbounded};
use rand::Rng;
use crate::audio_output::{AudioOutput, OutputState, PcmBuffer};

/// Max number of entries in played_list to prevent unbounded growth
const MAX_PLAYED_HISTORY: usize = 500;

/// Play mode enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    /// Loop through the entire playlist
    ListLoop,
    /// Play random tracks
    Random,
    /// Play in order, stop at the end
    List,
    /// Repeat the current track
    SingleLoop,
}

impl Default for PlayMode {
    fn default() -> Self {
        PlayMode::ListLoop
    }
}

/// Progress information
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProgressInfo {
    #[serde(rename = "currentTime")]
    pub current_time: u64,
    pub duration: u64,
}

impl Default for ProgressInfo {
    fn default() -> Self {
        ProgressInfo {
            current_time: 0,
            duration: 0,
        }
    }
}

/// Music item for internal use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicItem {
    pub id: String,
    pub name: String,
    pub singer: String,
    pub source: String,
    #[serde(rename = "albumId")]
    pub album_id: Option<String>,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    pub duration: Option<String>,
    #[serde(rename = "picUrl")]
    pub pic_url: Option<String>,
    #[serde(rename = "lrcUrl")]
    pub lrc_url: Option<String>,
    pub url: Option<String>,
}

impl From<crate::music_source::MusicInfo> for MusicItem {
    fn from(info: crate::music_source::MusicInfo) -> Self {
        MusicItem {
            id: info.id,
            name: info.name,
            singer: info.singer,
            source: info.source,
            album_id: info.album_id,
            album_name: info.album_name,
            duration: info.duration,
            pic_url: info.pic_url,
            lrc_url: info.lrc_url,
            url: info.url,
        }
    }
}

impl From<MusicItem> for crate::music_source::MusicInfo {
    fn from(item: MusicItem) -> Self {
        crate::music_source::MusicInfo {
            id: item.id,
            name: item.name,
            singer: item.singer,
            source: item.source,
            album_id: item.album_id,
            album_name: item.album_name,
            duration: item.duration,
            pic_url: item.pic_url,
            lrc_url: item.lrc_url,
            qualitys: vec![],
            url: item.url,
        }
    }
}

/// Player state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    #[serde(rename = "isPlaying")]
    pub is_playing: bool,
    #[serde(rename = "isPaused")]
    pub is_paused: bool,
    #[serde(rename = "currentMusic")]
    pub current_music: Option<MusicItem>,
    pub playlist: Vec<MusicItem>,
    #[serde(rename = "currentIndex")]
    pub current_index: usize,
    #[serde(rename = "playMode")]
    pub play_mode: PlayMode,
    pub progress: ProgressInfo,
    pub volume: f32,
    #[serde(rename = "playbackRate")]
    pub playback_rate: f32,
    #[serde(rename = "playedList")]
    pub played_list: Vec<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            is_playing: false,
            is_paused: false,
            current_music: None,
            playlist: Vec::new(),
            current_index: 0,
            play_mode: PlayMode::ListLoop,
            progress: ProgressInfo::default(),
            volume: 0.8,
            playback_rate: 1.0,
            played_list: Vec::new(),
        }
    }
}

/// Player command enumeration
#[derive(Debug)]
pub enum PlayerCommand {
    Play,
    Pause,
    Stop,
    TogglePlay,
    Next,
    Prev,
    Seek(u64),
    SetVolume(f32),
    SetPlaybackRate(f32),
    SetPlayMode(PlayMode),
    SetPlaylist(Vec<MusicItem>),
    PlayAtIndex(usize),
    AddToPlaylist(MusicItem),
    RemoveFromPlaylist(usize),
    ClearPlaylist,
}

/// Player event enumeration
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    StateChanged(PlayerState),
    PlaybackStarted,
    PlaybackPaused,
    PlaybackStopped,
    TrackChanged(MusicItem),
    ProgressUpdated(ProgressInfo),
    Error(String),
}

/// Player engine
pub struct PlayerEngine {
    state: Arc<RwLock<PlayerState>>,
    #[allow(dead_code)]
    command_tx: Option<Sender<PlayerCommand>>,
    audio_output: AudioOutput,
}

impl PlayerEngine {
    /// Create a new player engine
    pub fn new() -> Self {
        let state = Arc::new(RwLock::new(PlayerState::default()));
        let (command_tx, _command_rx) = unbounded();

        PlayerEngine {
            state,
            command_tx: Some(command_tx),
            audio_output: AudioOutput::new(),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> PlayerState {
        self.state.read().map(|guard| guard.clone()).unwrap_or_default()
    }

    /// Serialize state to JSON
    pub fn get_state_json(&self) -> String {
        let state = self.get_state();
        serde_json::to_string(&state).unwrap_or_else(|_| "{}".to_string())
    }

    /// Play
    pub fn play(&self) {
        let mut state = self.state.write().unwrap();
        if state.playlist.is_empty() {
            return;
        }
        
        if state.current_music.is_none() && !state.playlist.is_empty() {
            state.current_index = 0;
            state.current_music = Some(state.playlist[0].clone());
            state.progress = ProgressInfo { current_time: 0, duration: 0 };
        }
        
        state.is_playing = true;
        state.is_paused = false;
        self.audio_output.start();
    }

    /// Pause
    pub fn pause(&self) {
        let mut state = self.state.write().unwrap();
        state.is_playing = false;
        state.is_paused = true;
        self.audio_output.pause();
    }

    /// Stop
    pub fn stop(&self) {
        let mut state = self.state.write().unwrap();
        state.is_playing = false;
        state.is_paused = false;
        state.progress.current_time = 0;
        self.audio_output.stop();
    }

    /// Toggle play/pause
    pub fn toggle_play(&self) {
        let state = self.state.read().unwrap();
        if state.is_playing {
            drop(state);
            self.pause();
        } else {
            drop(state);
            self.play();
        }
    }

    /// Play next track
    pub fn next(&self) {
        let mut state = self.state.write().unwrap();
        if state.playlist.is_empty() {
            return;
        }

        let next_index = match state.play_mode {
            PlayMode::ListLoop => {
                if state.current_index >= state.playlist.len() - 1 {
                    0
                } else {
                    state.current_index + 1
                }
            }
            PlayMode::Random => {
                rand::thread_rng().gen_range(0..state.playlist.len())
            }
            PlayMode::List => {
                if state.current_index >= state.playlist.len() - 1 {
                    state.current_index
                } else {
                    state.current_index + 1
                }
            }
            PlayMode::SingleLoop => state.current_index,
        };

        state.current_index = next_index;
        state.current_music = Some(state.playlist[next_index].clone());
        state.progress = ProgressInfo { current_time: 0, duration: 0 };
        
        if let Some(id) = &state.current_music.as_ref().map(|m| m.id.clone()) {
            if state.played_list.len() >= MAX_PLAYED_HISTORY {
                state.played_list.remove(0);
            }
            state.played_list.push(id.clone());
        }
    }

    /// Play previous track
    pub fn prev(&self) {
        let mut state = self.state.write().unwrap();
        if state.playlist.is_empty() {
            return;
        }

        let prev_index = match state.play_mode {
            PlayMode::ListLoop | PlayMode::List => {
                if state.current_index == 0 {
                    state.playlist.len() - 1
                } else {
                    state.current_index - 1
                }
            }
            PlayMode::Random => {
                rand::thread_rng().gen_range(0..state.playlist.len())
            }
            PlayMode::SingleLoop => state.current_index,
        };

        state.current_index = prev_index;
        state.current_music = Some(state.playlist[prev_index].clone());
        state.progress = ProgressInfo { current_time: 0, duration: 0 };
        
        if let Some(id) = &state.current_music.as_ref().map(|m| m.id.clone()) {
            if state.played_list.len() >= MAX_PLAYED_HISTORY {
                state.played_list.remove(0);
            }
            state.played_list.push(id.clone());
        }
    }

    /// Seek to position
    pub fn seek(&self, time_ms: u64) {
        let mut state = self.state.write().unwrap();
        state.progress.current_time = time_ms;
    }

    /// Set volume (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) {
        let mut state = self.state.write().unwrap();
        let clamped = volume.clamp(0.0, 1.0);
        state.volume = clamped;
        self.audio_output.set_volume(clamped);
    }

    /// Set playback rate (0.5 - 2.0)
    pub fn set_playback_rate(&self, rate: f32) {
        let mut state = self.state.write().unwrap();
        let clamped = rate.clamp(0.5, 2.0);
        state.playback_rate = clamped;
        self.audio_output.set_playback_rate(clamped);
    }

    /// Set play mode
    pub fn set_play_mode(&self, mode: PlayMode) {
        let mut state = self.state.write().unwrap();
        state.play_mode = mode;
    }

    /// Set play mode from integer (0-3)
    pub fn set_play_mode_int(&self, mode: i32) {
        let play_mode = match mode {
            0 => PlayMode::ListLoop,
            1 => PlayMode::Random,
            2 => PlayMode::List,
            3 => PlayMode::SingleLoop,
            _ => PlayMode::ListLoop,
        };
        self.set_play_mode(play_mode);
    }

    /// Set playlist
    pub fn set_playlist(&self, playlist: Vec<MusicItem>) {
        let mut state = self.state.write().unwrap();
        state.playlist = playlist;
        state.current_index = 0;
        state.played_list.clear();
        
        if !state.playlist.is_empty() {
            state.current_music = Some(state.playlist[0].clone());
        } else {
            state.current_music = None;
        }
    }

    /// Play at specific index
    pub fn play_at_index(&self, index: usize) {
        let mut state = self.state.write().unwrap();
        if index >= state.playlist.len() {
            return;
        }
        
        state.current_index = index;
        state.current_music = Some(state.playlist[index].clone());
        state.progress = ProgressInfo { current_time: 0, duration: 0 };
        state.is_playing = true;
        state.is_paused = false;
        
        if let Some(id) = &state.current_music.as_ref().map(|m| m.id.clone()) {
            if state.played_list.len() >= MAX_PLAYED_HISTORY {
                state.played_list.remove(0);
            }
            state.played_list.push(id.clone());
        }
    }

    /// Add to playlist
    pub fn add_to_playlist(&self, music: MusicItem) {
        let mut state = self.state.write().unwrap();
        state.playlist.push(music);
    }

    /// Add to playlist from JSON
    pub fn add_to_playlist_json(&self, json: &str) -> Result<(), String> {
        let music: MusicItem = serde_json::from_str(json)
            .map_err(|e| e.to_string())?;
        self.add_to_playlist(music);
        Ok(())
    }

    /// Remove from playlist
    pub fn remove_from_playlist(&self, index: usize) {
        let mut state = self.state.write().unwrap();
        if index >= state.playlist.len() {
            return;
        }
        
        state.playlist.remove(index);
        
        if state.current_index >= state.playlist.len() && !state.playlist.is_empty() {
            state.current_index = state.playlist.len() - 1;
            state.current_music = Some(state.playlist[state.current_index].clone());
        }
    }

    /// Clear playlist
    pub fn clear_playlist(&self) {
        let mut state = self.state.write().unwrap();
        state.playlist.clear();
        state.current_music = None;
        state.current_index = 0;
        state.played_list.clear();
        state.is_playing = false;
        state.is_paused = false;
    }

    /// Set playlist from JSON
    pub fn set_playlist_json(&self, json: &str) -> Result<(), String> {
        let playlist: Vec<MusicItem> = serde_json::from_str(json)
            .map_err(|e| e.to_string())?;
        self.set_playlist(playlist);
        Ok(())
    }

    /// Update progress
    pub fn update_progress(&self, current_time: u64, duration: u64) {
        let mut state = self.state.write().unwrap();
        state.progress.current_time = current_time;
        state.progress.duration = duration;
    }

    // ========================================================================
    // Audio output methods
    // ========================================================================

    /// Queue PCM buffer for playback
    pub fn queue_audio_buffer(&self, samples: Vec<i16>, sample_rate: u32, channels: u16) {
        let buffer = PcmBuffer {
            samples,
            sample_rate,
            channels,
        };
        self.audio_output.queue_buffer(buffer);
    }

    /// Dequeue next PCM buffer for Android AudioTrack
    pub fn dequeue_audio_buffer(&self) -> Option<PcmBuffer> {
        self.audio_output.dequeue_buffer()
    }

    /// Get audio buffer count
    pub fn audio_buffer_count(&self) -> usize {
        self.audio_output.buffer_count()
    }

    /// Check if audio output is currently playing
    pub fn is_audio_playing(&self) -> bool {
        self.audio_output.is_playing()
    }

    /// Get audio output state
    pub fn get_audio_output_state(&self) -> OutputState {
        self.audio_output.get_state()
    }

    /// Get recommended audio buffer size in bytes
    pub fn audio_buffer_size_bytes(&self) -> usize {
        self.audio_output.buffer_size_bytes()
    }
}

impl Default for PlayerEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_music_item(id: &str, name: &str) -> MusicItem {
        MusicItem {
            id: id.to_string(),
            name: name.to_string(),
            singer: "Test".to_string(),
            source: "kw".to_string(),
            album_id: None,
            album_name: None,
            duration: None,
            pic_url: None,
            lrc_url: None,
            url: None,
        }
    }

    #[test]
    fn test_player_engine_new() {
        let engine = PlayerEngine::new();
        let state = engine.get_state();
        assert!(!state.is_playing);
        assert!(!state.is_paused);
        assert!(state.current_music.is_none());
        assert!(state.playlist.is_empty());
        assert_eq!(state.current_index, 0);
        assert_eq!(state.play_mode, PlayMode::ListLoop);
        assert_eq!(state.volume, 0.8);
        assert_eq!(state.playback_rate, 1.0);
    }

    #[test]
    fn test_play_with_empty_playlist() {
        let engine = PlayerEngine::new();
        engine.play();
        let state = engine.get_state();
        assert!(!state.is_playing); // empty playlist, should not play
    }

    #[test]
    fn test_set_playlist() {
        let engine = PlayerEngine::new();
        let playlist = vec![
            make_music_item("1", "song1"),
            make_music_item("2", "song2"),
            make_music_item("3", "song3"),
        ];
        engine.set_playlist(playlist);
        let state = engine.get_state();
        assert_eq!(state.playlist.len(), 3);
        assert_eq!(state.current_index, 0);
        assert!(state.current_music.is_some());
        assert_eq!(state.current_music.unwrap().id, "1");
    }

    #[test]
    fn test_set_playlist_empty() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![]);
        let state = engine.get_state();
        assert!(state.playlist.is_empty());
        assert!(state.current_music.is_none());
    }

    #[test]
    fn test_play() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "song1")]);
        engine.play();
        let state = engine.get_state();
        assert!(state.is_playing);
        assert!(!state.is_paused);
    }

    #[test]
    fn test_pause() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "song1")]);
        engine.play();
        engine.pause();
        let state = engine.get_state();
        assert!(!state.is_playing);
        assert!(state.is_paused);
    }

    #[test]
    fn test_stop() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "song1")]);
        engine.play();
        engine.stop();
        let state = engine.get_state();
        assert!(!state.is_playing);
        assert!(!state.is_paused);
        assert_eq!(state.progress.current_time, 0);
    }

    #[test]
    fn test_toggle_play() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "song1")]);
        // Initially stopped
        engine.toggle_play();
        assert!(engine.get_state().is_playing);
        engine.toggle_play();
        assert!(!engine.get_state().is_playing);
    }

    #[test]
    fn test_next_list_loop() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![
            make_music_item("1", "a"),
            make_music_item("2", "b"),
        ]);
        engine.set_play_mode(PlayMode::ListLoop);
        engine.next();
        assert_eq!(engine.get_state().current_index, 1);
        engine.next(); // wrap around
        assert_eq!(engine.get_state().current_index, 0);
    }

    #[test]
    fn test_next_list_mode() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![
            make_music_item("1", "a"),
            make_music_item("2", "b"),
        ]);
        engine.set_play_mode(PlayMode::List);
        engine.next();
        assert_eq!(engine.get_state().current_index, 1);
        engine.next(); // at end, should stay
        assert_eq!(engine.get_state().current_index, 1);
    }

    #[test]
    fn test_next_single_loop() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![
            make_music_item("1", "a"),
            make_music_item("2", "b"),
        ]);
        engine.set_play_mode(PlayMode::SingleLoop);
        engine.next();
        assert_eq!(engine.get_state().current_index, 0); // stays on same track
    }

    #[test]
    fn test_next_empty_playlist() {
        let engine = PlayerEngine::new();
        engine.next(); // should not panic
        let state = engine.get_state();
        assert!(state.current_music.is_none());
    }

    #[test]
    fn test_prev_list_loop() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![
            make_music_item("1", "a"),
            make_music_item("2", "b"),
        ]);
        engine.set_play_mode(PlayMode::ListLoop);
        engine.prev(); // wrap to end
        assert_eq!(engine.get_state().current_index, 1);
    }

    #[test]
    fn test_seek() {
        let engine = PlayerEngine::new();
        engine.seek(12345);
        assert_eq!(engine.get_state().progress.current_time, 12345);
    }

    #[test]
    fn test_set_volume() {
        let engine = PlayerEngine::new();
        engine.set_volume(0.5);
        assert_eq!(engine.get_state().volume, 0.5);
        // Clamping
        engine.set_volume(1.5);
        assert_eq!(engine.get_state().volume, 1.0);
        engine.set_volume(-0.5);
        assert_eq!(engine.get_state().volume, 0.0);
    }

    #[test]
    fn test_set_playback_rate() {
        let engine = PlayerEngine::new();
        engine.set_playback_rate(1.5);
        assert_eq!(engine.get_state().playback_rate, 1.5);
        // Clamping
        engine.set_playback_rate(3.0);
        assert_eq!(engine.get_state().playback_rate, 2.0);
        engine.set_playback_rate(0.1);
        assert_eq!(engine.get_state().playback_rate, 0.5);
    }

    #[test]
    fn test_set_play_mode() {
        let engine = PlayerEngine::new();
        engine.set_play_mode(PlayMode::Random);
        assert_eq!(engine.get_state().play_mode, PlayMode::Random);
    }

    #[test]
    fn test_set_play_mode_int() {
        let engine = PlayerEngine::new();
        engine.set_play_mode_int(0);
        assert_eq!(engine.get_state().play_mode, PlayMode::ListLoop);
        engine.set_play_mode_int(1);
        assert_eq!(engine.get_state().play_mode, PlayMode::Random);
        engine.set_play_mode_int(2);
        assert_eq!(engine.get_state().play_mode, PlayMode::List);
        engine.set_play_mode_int(3);
        assert_eq!(engine.get_state().play_mode, PlayMode::SingleLoop);
        engine.set_play_mode_int(99); // invalid
        assert_eq!(engine.get_state().play_mode, PlayMode::ListLoop);
    }

    #[test]
    fn test_play_at_index() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![
            make_music_item("1", "a"),
            make_music_item("2", "b"),
            make_music_item("3", "c"),
        ]);
        engine.play_at_index(2);
        let state = engine.get_state();
        assert_eq!(state.current_index, 2);
        assert!(state.is_playing);
        assert_eq!(state.current_music.unwrap().id, "3");
    }

    #[test]
    fn test_play_at_index_out_of_bounds() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "a")]);
        engine.play_at_index(99);
        let state = engine.get_state();
        // Index should be unchanged
        assert_eq!(state.current_index, 0);
    }

    #[test]
    fn test_add_to_playlist() {
        let engine = PlayerEngine::new();
        engine.add_to_playlist(make_music_item("1", "a"));
        engine.add_to_playlist(make_music_item("2", "b"));
        assert_eq!(engine.get_state().playlist.len(), 2);
    }

    #[test]
    fn test_remove_from_playlist() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![
            make_music_item("1", "a"),
            make_music_item("2", "b"),
            make_music_item("3", "c"),
        ]);
        engine.remove_from_playlist(1);
        let state = engine.get_state();
        assert_eq!(state.playlist.len(), 2);
        assert_eq!(state.playlist[0].id, "1");
        assert_eq!(state.playlist[1].id, "3");
    }

    #[test]
    fn test_remove_from_playlist_out_of_bounds() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "a")]);
        engine.remove_from_playlist(99);
        assert_eq!(engine.get_state().playlist.len(), 1);
    }

    #[test]
    fn test_clear_playlist() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "a")]);
        engine.clear_playlist();
        let state = engine.get_state();
        assert!(state.playlist.is_empty());
        assert!(state.current_music.is_none());
        assert!(!state.is_playing);
    }

    #[test]
    fn test_update_progress() {
        let engine = PlayerEngine::new();
        engine.update_progress(5000, 300000);
        let state = engine.get_state();
        assert_eq!(state.progress.current_time, 5000);
        assert_eq!(state.progress.duration, 300000);
    }

    #[test]
    fn test_get_state_json() {
        let engine = PlayerEngine::new();
        let json = engine.get_state_json();
        assert!(json.contains("isPlaying"));
        assert!(json.contains("playMode"));
    }

    #[test]
    fn test_set_playlist_json() {
        let engine = PlayerEngine::new();
        let json = r#"[{"id":"1","name":"song","singer":"x","source":"kw"}]"#;
        let result = engine.set_playlist_json(json);
        assert!(result.is_ok());
        assert_eq!(engine.get_state().playlist.len(), 1);
    }

    #[test]
    fn test_set_playlist_json_invalid() {
        let engine = PlayerEngine::new();
        let result = engine.set_playlist_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_add_to_playlist_json() {
        let engine = PlayerEngine::new();
        let json = r#"{"id":"1","name":"song","singer":"x","source":"kw"}"#;
        let result = engine.add_to_playlist_json(json);
        assert!(result.is_ok());
        assert_eq!(engine.get_state().playlist.len(), 1);
    }

    #[test]
    fn test_music_item_from_music_info() {
        let info = crate::music_source::MusicInfo {
            id: "1".to_string(),
            name: "s".to_string(),
            singer: "a".to_string(),
            source: "kw".to_string(),
            album_id: None,
            album_name: None,
            duration: None,
            pic_url: None,
            lrc_url: None,
            qualitys: vec![],
            url: None,
        };
        let item = MusicItem::from(info);
        assert_eq!(item.id, "1");
        assert_eq!(item.name, "s");
    }

    #[test]
    fn test_play_mode_default() {
        assert_eq!(PlayMode::default(), PlayMode::ListLoop);
    }

    #[test]
    fn test_progress_info_default() {
        let pi = ProgressInfo::default();
        assert_eq!(pi.current_time, 0);
        assert_eq!(pi.duration, 0);
    }

    #[test]
    fn test_player_state_default() {
        let state = PlayerState::default();
        assert!(!state.is_playing);
        assert!(!state.is_paused);
        assert!(state.current_music.is_none());
        assert!(state.playlist.is_empty());
        assert_eq!(state.play_mode, PlayMode::ListLoop);
        assert_eq!(state.volume, 0.8);
        assert_eq!(state.playback_rate, 1.0);
    }

    #[test]
    fn test_audio_buffer_queue() {
        let engine = PlayerEngine::new();
        engine.set_playlist(vec![make_music_item("1", "a")]);
        engine.play(); // must be playing to queue buffers
        let samples = vec![1i16, 2, 3, 4];
        engine.queue_audio_buffer(samples, 44100, 2);
        assert_eq!(engine.audio_buffer_count(), 1);
        let buf = engine.dequeue_audio_buffer();
        assert!(buf.is_some());
        let buf = buf.unwrap();
        assert_eq!(buf.samples.len(), 4);
        assert_eq!(buf.sample_rate, 44100);
        assert_eq!(buf.channels, 2);
    }

    #[test]
    fn test_dequeue_empty() {
        let engine = PlayerEngine::new();
        assert!(engine.dequeue_audio_buffer().is_none());
    }

    #[test]
    fn test_audio_buffer_size_bytes() {
        let engine = PlayerEngine::new();
        assert!(engine.audio_buffer_size_bytes() > 0);
    }
}