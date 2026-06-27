//! Audio Output Module
//!
//! Manages audio output via Android AudioTrack through JNI.
//! Receives decoded PCM data and pushes it to the Android audio subsystem.

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::collections::VecDeque;

/// Audio output configuration
#[derive(Debug, Clone)]
pub struct AudioOutputConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size_ms: u32,
}

impl Default for AudioOutputConfig {
    fn default() -> Self {
        AudioOutputConfig {
            sample_rate: 44100,
            channels: 2,
            buffer_size_ms: 100,
        }
    }
}

/// State of the audio output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputState {
    Idle,
    Playing,
    Paused,
    Stopped,
}

/// PCM buffer chunk for audio output
#[derive(Debug, Clone)]
pub struct PcmBuffer {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Audio output engine
///
/// Manages the lifecycle of audio playback output.
/// On Android, this connects to AudioTrack via JNI callbacks.
/// The actual JNI AudioTrack implementation lives in the Android layer,
/// while this module provides the Rust-side state management and buffer
/// queue that feeds data to the JNI bridge.
pub struct AudioOutput {
    config: AudioOutputConfig,
    state: Arc<Mutex<OutputState>>,
    playing: Arc<AtomicBool>,
    buffer_queue: Arc<Mutex<VecDeque<PcmBuffer>>>,
    volume: Arc<Mutex<f32>>,
    playback_rate: Arc<Mutex<f32>>,
}

impl AudioOutput {
    pub fn new() -> Self {
        AudioOutput {
            config: AudioOutputConfig::default(),
            state: Arc::new(Mutex::new(OutputState::Idle)),
            playing: Arc::new(AtomicBool::new(false)),
            buffer_queue: Arc::new(Mutex::new(VecDeque::new())),
            volume: Arc::new(Mutex::new(0.8)),
            playback_rate: Arc::new(Mutex::new(1.0)),
        }
    }

    pub fn with_config(config: AudioOutputConfig) -> Self {
        AudioOutput {
            config,
            ..Default::default()
        }
    }

    /// Get current output state
    pub fn get_state(&self) -> OutputState {
        *self.state.lock().unwrap()
    }

    /// Check if audio is currently playing
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::SeqCst)
    }

    /// Start playback
    pub fn start(&self) {
        *self.state.lock().unwrap() = OutputState::Playing;
        self.playing.store(true, Ordering::SeqCst);
    }

    /// Pause playback
    pub fn pause(&self) {
        *self.state.lock().unwrap() = OutputState::Paused;
        self.playing.store(false, Ordering::SeqCst);
    }

    /// Resume playback
    pub fn resume(&self) {
        if *self.state.lock().unwrap() == OutputState::Paused {
            *self.state.lock().unwrap() = OutputState::Playing;
            self.playing.store(true, Ordering::SeqCst);
        }
    }

    /// Stop playback and clear buffers
    pub fn stop(&self) {
        *self.state.lock().unwrap() = OutputState::Stopped;
        self.playing.store(false, Ordering::SeqCst);
        self.buffer_queue.lock().unwrap().clear();
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) {
        *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }

    /// Get current volume
    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    /// Set playback rate (0.5 to 2.0)
    pub fn set_playback_rate(&self, rate: f32) {
        *self.playback_rate.lock().unwrap() = rate.clamp(0.5, 2.0);
    }

    /// Get current playback rate
    pub fn get_playback_rate(&self) -> f32 {
        *self.playback_rate.lock().unwrap()
    }

    /// Queue PCM data for playback
    pub fn queue_buffer(&self, buffer: PcmBuffer) {
        if self.is_playing() {
            self.buffer_queue.lock().unwrap().push_back(buffer);
        }
    }

    /// Dequeue the next PCM buffer for consumption by Android AudioTrack
    pub fn dequeue_buffer(&self) -> Option<PcmBuffer> {
        self.buffer_queue.lock().unwrap().pop_front()
    }

    /// Get number of queued buffers
    pub fn buffer_count(&self) -> usize {
        self.buffer_queue.lock().unwrap().len()
    }

    /// Check if buffer queue is empty
    pub fn is_buffer_empty(&self) -> bool {
        self.buffer_queue.lock().unwrap().is_empty()
    }

    /// Get recommended buffer size in bytes
    /// For 44100Hz stereo 16-bit: 100ms = 44100 * 2 * 2 * 0.1 = 17640 bytes
    pub fn buffer_size_bytes(&self) -> usize {
        let samples_per_ms = self.config.sample_rate as f64 / 1000.0;
        let bytes_per_sample = 2; // i16
        (samples_per_ms * self.config.buffer_size_ms as f64
            * self.config.channels as f64
            * bytes_per_sample as f64) as usize
    }

    /// Get recommended buffer size in samples
    pub fn buffer_size_samples(&self) -> usize {
        let samples_per_ms = self.config.sample_rate as f64 / 1000.0;
        (samples_per_ms * self.config.buffer_size_ms as f64
            * self.config.channels as f64) as usize
    }
}

impl Default for AudioOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_output_lifecycle() {
        let output = AudioOutput::new();
        assert_eq!(output.get_state(), OutputState::Idle);
        assert!(!output.is_playing());

        output.start();
        assert_eq!(output.get_state(), OutputState::Playing);
        assert!(output.is_playing());

        output.pause();
        assert_eq!(output.get_state(), OutputState::Paused);
        assert!(!output.is_playing());

        output.resume();
        assert_eq!(output.get_state(), OutputState::Playing);
        assert!(output.is_playing());

        output.stop();
        assert_eq!(output.get_state(), OutputState::Stopped);
        assert!(!output.is_playing());
    }

    #[test]
    fn test_buffer_queue() {
        let output = AudioOutput::new();
        output.start();

        let buffer = PcmBuffer {
            samples: vec![0i16; 100],
            sample_rate: 44100,
            channels: 2,
        };

        output.queue_buffer(buffer);
        assert_eq!(output.buffer_count(), 1);
        assert!(!output.is_buffer_empty());

        let dequeued = output.dequeue_buffer();
        assert!(dequeued.is_some());
        assert_eq!(output.buffer_count(), 0);
        assert!(output.is_buffer_empty());
    }

    #[test]
    fn test_volume() {
        let output = AudioOutput::new();
        assert_eq!(output.get_volume(), 0.8);

        output.set_volume(0.5);
        assert_eq!(output.get_volume(), 0.5);

        output.set_volume(1.5); // Clamped
        assert_eq!(output.get_volume(), 1.0);

        output.set_volume(-0.1); // Clamped
        assert_eq!(output.get_volume(), 0.0);
    }

    #[test]
    fn test_buffer_size() {
        let output = AudioOutput::with_config(AudioOutputConfig {
            sample_rate: 44100,
            channels: 2,
            buffer_size_ms: 100,
        });

        let bytes = output.buffer_size_bytes();
        assert_eq!(bytes, 17640); // 44100 * 2 * 2 * 0.1

        let samples = output.buffer_size_samples();
        assert_eq!(samples, 8820); // 44100 * 2 * 0.1
    }
}