//! LX Music Core Library
//! 
//! Rust implementation of core music functionality:
//! - Music source management and parsing
//! - Music search and URL resolution
//! - Playback engine (state management)
//! - Lyric parsing and synchronization
//! 
//! This library is designed to be called via JNI from TurboModule (C++).

pub mod music_source;
pub mod http_utils;
pub mod crypto_utils;
pub mod player;
pub mod lyric;
pub mod sources;
pub mod audio_decoder;
pub mod audio_output;

#[cfg(feature = "js-engine")]
pub mod js_engine;

#[cfg(target_os = "android")]
pub mod jni_bridge;

pub use music_source::{SourceInfo, MusicInfo, SearchResult, LyricInfo, QualityInfo};
pub use http_utils::HttpClient;
pub use crypto_utils::CryptoUtils;
pub use player::{PlayerEngine, PlayerState, PlayMode, ProgressInfo, MusicItem};
pub use lyric::{LyricEngine, LyricData, LyricLine};
pub use sources::{get_source_manager, SourceManager, MusicSourceApi};
pub use audio_decoder::AudioDecoder;
pub use audio_output::{AudioOutput, AudioOutputConfig, OutputState, PcmBuffer};

#[cfg(feature = "js-engine")]
pub use js_engine::JsEngine;