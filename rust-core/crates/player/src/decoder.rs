use common::Result;
use symphonia::core::io::MediaSource;

/// 音频解码器 trait
pub trait AudioDecoder: Send + Sync {
    /// 打开音频源
    fn open(&mut self, source: Box<dyn MediaSource>) -> Result<()>;
    
    /// 解码下一帧
    fn decode_frame(&mut self) -> Result<Option<AudioFrame>>;
    
    /// 获取采样率
    fn sample_rate(&self) -> u32;
    
    /// 获取通道数
    fn channels(&self) -> u16;
    
    /// 获取总时长（秒）
    fn duration(&self) -> Option<f64>;
    
    /// 跳转到指定位置
    fn seek(&mut self, position: f64) -> Result<()>;
}

/// 音频帧
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub data: Vec<f32>,
    pub timestamp: f64,
}

/// Symphonia 解码器实现
pub struct SymphoniaDecoder {
    // Symphonia 内部状态
    reader: Option<symphonia::core::io::MediaSourceStream>,
    format: Option<Box<dyn symphonia::core::formats::FormatReader>>,
    decoder: Option<Box<dyn symphonia::core::codecs::Decoder>>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
}

impl SymphoniaDecoder {
    pub fn new() -> Self {
        Self {
            reader: None,
            format: None,
            decoder: None,
            track_id: 0,
            sample_rate: 44100,
            channels: 2,
        }
    }
}

impl AudioDecoder for SymphoniaDecoder {
    fn open(&mut self, source: Box<dyn MediaSource>) -> Result<()> {
        use symphonia::core::formats::{FormatOptions, FormatReader};
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;
        
        let mss = MediaSourceStream::new(source, Default::default());
        let hint = Hint::new();
        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        
        // 探测格式
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| common::Error::Player(e.to_string()))?;
        
        let format = probed.format;
        let track = format.tracks()
            .first()
            .ok_or_else(|| common::Error::Player("No audio track found".to_string()))?;
        
        self.track_id = track.id;
        self.sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        self.channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        
        // 创建解码器
        let decoder_opts: symphonia::core::codecs::DecoderOptions = Default::default();
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .map_err(|e| common::Error::Player(e.to_string()))?;
        
        self.decoder = Some(decoder);
        self.format = Some(format);
        
        Ok(())
    }
    
    fn decode_frame(&mut self) -> Result<Option<AudioFrame>> {
        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::Decoder;
        use symphonia::core::formats::FormatReader;
        
        let format = self.format.as_mut()
            .ok_or_else(|| common::Error::Player("Format not initialized".to_string()))?;
        let decoder = self.decoder.as_mut()
            .ok_or_else(|| common::Error::Player("Decoder not initialized".to_string()))?;
        
        // 读取下一个包
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(_)) => return Ok(None),
            Err(e) => return Err(common::Error::Player(e.to_string())),
        };
        
        // 解码
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // 转换为 f32 样本
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
                sample_buf.copy_interleaved_ref(decoded);
                
                let timestamp = packet.ts as f64 / self.sample_rate as f64;
                
                Ok(Some(AudioFrame {
                    data: sample_buf.samples().to_vec(),
                    timestamp,
                }))
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => Ok(None),
            Err(e) => Err(common::Error::Player(e.to_string())),
        }
    }
    
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    fn channels(&self) -> u16 {
        self.channels
    }
    
    fn duration(&self) -> Option<f64> {
        // 从 format 获取时长
        None
    }
    
    fn seek(&mut self, position: f64) -> Result<()> {
        use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
        
        if let Some(format) = self.format.as_mut() {
            let time = symphonia::core::units::Time::new(
                position as u64,
                (position.fract() * 1_000_000_000.0) as u32 as f64,
            );
            
            format.seek(
                SeekMode::Coarse,
                SeekTo::Time { track_id: Some(self.track_id), time },
            ).map_err(|e| common::Error::Player(e.to_string()))?;
        }
        
        Ok(())
    }
}

/// 解码器工厂
pub struct DecoderFactory;

impl DecoderFactory {
    /// 根据文件扩展名创建对应的解码器
    pub fn create_by_extension(ext: &str) -> Result<Box<dyn AudioDecoder>> {
        match ext.to_lowercase().as_str() {
            "mp3" | "flac" | "ogg" | "m4a" | "aac" | "wav" => {
                Ok(Box::new(SymphoniaDecoder::new()))
            }
            _ => Err(common::Error::Player(
                format!("Unsupported audio format: {}", ext)
            )),
        }
    }
}
