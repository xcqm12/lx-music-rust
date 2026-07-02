use common::{Error, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use super::decoder::{AudioDecoder, DecoderFactory};

#[derive(Debug, Clone)]
pub enum AudioEvent {
    Buffering(f32),
    PlaybackStarted,
    PlaybackPaused,
    PlaybackStopped,
    PositionChanged(f64),
    DurationChanged(f64),
    Error(String),
    Completed,
}

struct AudioState {
    is_playing: bool,
    is_paused: bool,
    position: f64,
    duration: f64,
    buffered: f64,
    volume: f32,
    sample_rate: u32,
    channels: u16,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_paused: false,
            position: 0.0,
            duration: 0.0,
            buffered: 0.0,
            volume: 1.0,
            sample_rate: 44100,
            channels: 2,
        }
    }
}

pub struct AudioEngine {
    state: Arc<RwLock<AudioState>>,
    event_sender: mpsc::Sender<AudioEvent>,
    event_receiver: Option<mpsc::Receiver<AudioEvent>>,
    command_sender: mpsc::Sender<EngineCommand>,
    _decoder_thread: Option<std::thread::JoinHandle<()>>,
}

#[derive(Debug)]
enum EngineCommand {
    PlayUrl(String),
    Pause,
    Resume,
    Stop,
    Seek(f64),
    SetVolume(f32),
    Shutdown,
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::channel(100);
        let (command_sender, command_receiver) = mpsc::channel(50);
        let state = Arc::new(RwLock::new(AudioState::default()));

        let engine = Self {
            state: state.clone(),
            event_sender: event_sender.clone(),
            event_receiver: Some(event_receiver),
            command_sender,
            _decoder_thread: None,
        };

        Self::start_command_loop(command_receiver, state, event_sender);

        Ok(engine)
    }

    fn start_command_loop(
        mut command_receiver: mpsc::Receiver<EngineCommand>,
        state: Arc<RwLock<AudioState>>,
        event_sender: mpsc::Sender<AudioEvent>,
    ) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async move {
                let mut decoder: Option<Box<dyn AudioDecoder>> = None;
                let mut buffer: Vec<f32> = Vec::new();
                let mut is_eof = false;

                loop {
                    tokio::select! {
                        cmd = command_receiver.recv() => {
                            match cmd {
                                Some(EngineCommand::PlayUrl(url)) => {
                                    is_eof = false;
                                    buffer.clear();

                                    let ext = Self::get_extension(&url);
                                    match DecoderFactory::create_by_extension(&ext) {
                                        Ok(mut dec) => {
                                            match Self::open_source(&mut dec, &url).await {
                                                Ok(()) => {
                                                    let sr = dec.sample_rate();
                                                    let ch = dec.channels();
                                                    {
                                                        let mut s = state.write().await;
                                                        s.sample_rate = sr;
                                                        s.channels = ch;
                                                        s.position = 0.0;
                                                        s.duration = dec.duration().unwrap_or(0.0);
                                                        s.is_playing = true;
                                                        s.is_paused = false;
                                                    }
                                                    event_sender.send(AudioEvent::PlaybackStarted).await.ok();
                                                    event_sender.send(AudioEvent::DurationChanged(dec.duration().unwrap_or(0.0))).await.ok();
                                                    decoder = Some(dec);
                                                }
                                                Err(e) => {
                                                    event_sender.send(AudioEvent::Error(e.to_string())).await.ok();
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            event_sender.send(AudioEvent::Error(e.to_string())).await.ok();
                                        }
                                    }
                                }
                                Some(EngineCommand::Pause) => {
                                    let mut s = state.write().await;
                                    if s.is_playing && !s.is_paused {
                                        s.is_paused = true;
                                        event_sender.send(AudioEvent::PlaybackPaused).await.ok();
                                    }
                                }
                                Some(EngineCommand::Resume) => {
                                    let mut s = state.write().await;
                                    if s.is_paused {
                                        s.is_paused = false;
                                        event_sender.send(AudioEvent::PlaybackStarted).await.ok();
                                    }
                                }
                                Some(EngineCommand::Stop) => {
                                    let mut s = state.write().await;
                                    s.is_playing = false;
                                    s.is_paused = false;
                                    s.position = 0.0;
                                    s.buffered = 0.0;
                                    decoder = None;
                                    buffer.clear();
                                    is_eof = false;
                                    event_sender.send(AudioEvent::PlaybackStopped).await.ok();
                                }
                                Some(EngineCommand::Seek(position)) => {
                                    if let Some(ref mut dec) = decoder {
                                        if let Err(e) = dec.seek(position) {
                                            event_sender.send(AudioEvent::Error(e.to_string())).await.ok();
                                        } else {
                                            buffer.clear();
                                            is_eof = false;
                                            let mut s = state.write().await;
                                            s.position = position;
                                            event_sender.send(AudioEvent::PositionChanged(position)).await.ok();
                                        }
                                    }
                                }
                                Some(EngineCommand::SetVolume(vol)) => {
                                    let mut s = state.write().await;
                                    s.volume = vol.clamp(0.0, 1.0);
                                }
                                Some(EngineCommand::Shutdown) => {
                                    break;
                                }
                                None => break,
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_millis(10)) => {
                            let is_playing = {
                                let s = state.read().await;
                                s.is_playing && !s.is_paused
                            };

                            if is_playing && !is_eof {
                                if let Some(ref mut dec) = decoder {
                                    if buffer.len() < 65536 {
                                        match dec.decode_frame() {
                                            Ok(Some(frame)) => {
                                                buffer.extend_from_slice(&frame.data);
                                                let sr = dec.sample_rate() as f64;
                                                let ch = dec.channels() as f64;
                                                let buffered = buffer.len() as f64 / (sr * ch);
                                                {
                                                    let mut s = state.write().await;
                                                    s.buffered = buffered;
                                                }
                                                event_sender.send(AudioEvent::Buffering(buffered.min(1.0) as f32)).await.ok();
                                            }
                                            Ok(None) => {
                                                is_eof = true;
                                                if buffer.is_empty() {
                                                    let mut s = state.write().await;
                                                    s.is_playing = false;
                                                    event_sender.send(AudioEvent::Completed).await.ok();
                                                }
                                            }
                                            Err(e) => {
                                                event_sender.send(AudioEvent::Error(e.to_string())).await.ok();
                                                is_eof = true;
                                            }
                                        }
                                    }
                                }
                            }

                            if is_playing && !buffer.is_empty() {
                                let s = state.read().await;
                                let sr = s.sample_rate as f64;
                                let ch = s.channels as f64;
                                let advance = 0.01 * sr * ch;
                                drop(s);

                                let samples_to_advance = advance as usize;
                                if buffer.len() >= samples_to_advance {
                                    buffer.drain(0..samples_to_advance);
                                    let mut s = state.write().await;
                                    s.position += 0.01;
                                    event_sender.send(AudioEvent::PositionChanged(s.position)).await.ok();
                                } else if is_eof {
                                    buffer.clear();
                                    let mut s = state.write().await;
                                    s.is_playing = false;
                                    s.position = s.duration;
                                    event_sender.send(AudioEvent::Completed).await.ok();
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    fn get_extension(url: &str) -> String {
        let path = url.split('?').next().unwrap_or(url);
        path.split('.')
            .last()
            .unwrap_or("mp3")
            .to_lowercase()
    }

    async fn open_source(decoder: &mut Box<dyn AudioDecoder>, url: &str) -> Result<()> {
        if url.starts_with("http://") || url.starts_with("https://") {
            let response = reqwest::get(url)
                .await
                .map_err(|e| Error::Player(format!("HTTP request failed: {}", e)))?;

            let bytes = response
                .bytes()
                .await
                .map_err(|e| Error::Player(format!("Failed to download: {}", e)))?;

            let cursor = std::io::Cursor::new(bytes.to_vec());
            decoder.open(Box::new(cursor))
        } else {
            let file = std::fs::File::open(url)
                .map_err(|e| Error::Player(format!("File open failed: {}", e)))?;
            decoder.open(Box::new(file))
        }
    }

    pub async fn play_url(&self, url: &str) -> Result<()> {
        self.command_sender
            .send(EngineCommand::PlayUrl(url.to_string()))
            .await
            .map_err(|e| Error::Player(format!("Command send failed: {}", e)))?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        self.command_sender
            .send(EngineCommand::Pause)
            .await
            .map_err(|e| Error::Player(format!("Command send failed: {}", e)))?;
        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        self.command_sender
            .send(EngineCommand::Resume)
            .await
            .map_err(|e| Error::Player(format!("Command send failed: {}", e)))?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.command_sender
            .send(EngineCommand::Stop)
            .await
            .map_err(|e| Error::Player(format!("Command send failed: {}", e)))?;
        Ok(())
    }

    pub async fn seek(&self, position: f64) -> Result<()> {
        self.command_sender
            .send(EngineCommand::Seek(position))
            .await
            .map_err(|e| Error::Player(format!("Command send failed: {}", e)))?;
        Ok(())
    }

    pub async fn set_volume(&self, volume: f32) {
        let _ = self.command_sender.send(EngineCommand::SetVolume(volume)).await;
    }

    pub async fn get_volume(&self) -> f32 {
        self.state.read().await.volume
    }

    pub async fn get_position(&self) -> f64 {
        self.state.read().await.position
    }

    pub async fn get_duration(&self) -> f64 {
        self.state.read().await.duration
    }

    pub async fn get_buffered(&self) -> f64 {
        self.state.read().await.buffered
    }

    pub async fn is_playing(&self) -> bool {
        let s = self.state.read().await;
        s.is_playing && !s.is_paused
    }

    pub async fn is_paused(&self) -> bool {
        self.state.read().await.is_paused
    }

    pub fn get_event_receiver(&mut self) -> Option<mpsc::Receiver<AudioEvent>> {
        self.event_receiver.take()
    }
}

// 当 AudioEngine 被 drop 时，command_sender 会被自动丢弃，
// command_receiver.recv() 会返回 None，循环自动退出

pub struct AudioBuffer {
    data: Vec<f32>,
    write_pos: usize,
    read_pos: usize,
    total_samples: usize,
    sample_rate: u32,
}

impl AudioBuffer {
    pub fn new(capacity: usize, sample_rate: u32) -> Self {
        Self {
            data: vec![0.0; capacity],
            write_pos: 0,
            read_pos: 0,
            total_samples: 0,
            sample_rate,
        }
    }

    pub fn write(&mut self, data: &[f32]) {
        for &sample in data {
            if self.write_pos < self.data.len() {
                self.data[self.write_pos] = sample;
                self.write_pos += 1;
            }
        }
        self.total_samples += data.len();
    }

    pub fn read(&mut self, buf: &mut [f32]) -> usize {
        let mut count = 0;
        for item in buf.iter_mut() {
            if self.read_pos < self.write_pos {
                *item = self.data[self.read_pos];
                self.read_pos += 1;
                count += 1;
            } else {
                *item = 0.0;
            }
        }
        count
    }

    pub fn position(&self) -> f64 {
        let channels = 2u32;
        let played_samples = self.read_pos;
        played_samples as f64 / (self.sample_rate * channels) as f64
    }

    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.read_pos = 0;
        self.total_samples = 0;
    }

    pub fn len(&self) -> usize {
        self.write_pos - self.read_pos
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
