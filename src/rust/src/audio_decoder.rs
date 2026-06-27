//! Audio Decoder Module
//!
//! Uses Symphonia to decode audio data from various formats
//! (MP3, AAC/M4A, FLAC, WAV) into raw PCM samples.

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Audio format information after probing
#[derive(Debug, Clone)]
pub struct AudioFormat {
    /// Codec name (e.g. "mp3", "aac", "flac", "pcm")
    pub codec: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Total duration in seconds
    pub duration_secs: f64,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Average bitrate in kbps
    pub bitrate_kbps: u32,
    /// Total decoded PCM frames
    pub total_frames: u64,
}

/// Decoded PCM audio data
pub struct DecodedAudio {
    /// Interleaved i16 PCM samples
    pub samples: Vec<i16>,
    /// Sample rate
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Audio decoder based on Symphonia
pub struct AudioDecoder;

impl AudioDecoder {
    pub fn new() -> Self {
        AudioDecoder
    }

    /// Probe audio data and return format information without full decoding
    pub fn probe_format(data: &[u8]) -> Result<AudioFormat, String> {
        let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(data.to_vec())), Default::default());

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let hint = Hint::new();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| format!("Probe failed: {}", e))?;

        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("No audio track found")?;

        let codec_params = &track.codec_params;

        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);

        let time_base = codec_params.time_base;
        let n_frames = codec_params.n_frames.unwrap_or(0);

        let duration_secs = if let Some(tb) = time_base {
            n_frames as f64 * tb.numer as f64 / tb.denom as f64
        } else {
            0.0
        };

        let duration_ms = (duration_secs * 1000.0) as u64;

        let bitrate_kbps = if duration_secs > 0.0 {
            (data.len() as u64 * 8 / 1000) as f64 / duration_secs
        } else {
            0.0
        } as u32;

        let codec_name = format!("{:?}", codec_params.codec);

        Ok(AudioFormat {
            codec: codec_name,
            sample_rate,
            channels,
            duration_secs,
            duration_ms,
            bitrate_kbps,
            total_frames: n_frames,
        })
    }

    /// Decode audio data and return PCM samples
    /// Returns interleaved i16 PCM samples at the original sample rate
    pub fn decode(data: &[u8]) -> Result<DecodedAudio, String> {
        let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(data.to_vec())), Default::default());

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let hint = Hint::new();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| format!("Probe failed: {}", e))?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("No audio track found")?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);

        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Decoder creation failed: {}", e))?;

        let mut all_samples = Vec::new();
        let mut total_frames = 0u64;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(SymphoniaError::ResetRequired) => {
                    // Codec needs reset, create new decoder
                    decoder = symphonia::default::get_codecs()
                        .make(&codec_params, &DecoderOptions::default())
                        .map_err(|e| format!("Decoder reset failed: {}", e))?;
                    continue;
                }
                Err(e) => {
                    return Err(format!("Packet read error: {}", e));
                }
            };

            if packet.track_id() != track_id {
                continue;
            }

            let decoded = match decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => return Err(format!("Decode error: {}", e)),
            };

            let spec = *decoded.spec();
            let num_frames = decoded.frames();
            total_frames += num_frames as u64;

            // Convert to i16 PCM
            let mut sample_buf = SampleBuffer::<i16>::new(num_frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            all_samples.extend_from_slice(sample_buf.samples());
        }

        let duration_ms = if sample_rate > 0 {
            (total_frames * 1000) / sample_rate as u64
        } else {
            0
        };

        Ok(DecodedAudio {
            samples: all_samples,
            sample_rate,
            channels,
            duration_ms,
        })
    }

    /// Decode audio data and resample to target sample rate
    pub fn decode_resampled(data: &[u8], target_sample_rate: u32) -> Result<DecodedAudio, String> {
        let decoded = Self::decode(data)?;

        if decoded.sample_rate == target_sample_rate {
            return Ok(decoded);
        }

        // Simple linear interpolation resampling
        let ratio = decoded.sample_rate as f64 / target_sample_rate as f64;
        let output_len = ((decoded.samples.len() as f64 / ratio) as usize / decoded.channels as usize) * decoded.channels as usize;
        let mut resampled = Vec::with_capacity(output_len);

        for ch in 0..decoded.channels as usize {
            for i in 0..output_len / decoded.channels as usize {
                let src_idx = (i as f64 * ratio) as usize * decoded.channels as usize + ch;
                let next_idx = (src_idx + decoded.channels as usize).min(decoded.samples.len() - 1);

                if src_idx < decoded.samples.len() {
                    let frac = (i as f64 * ratio).fract();
                    let sample = decoded.samples[src_idx] as f64 * (1.0 - frac)
                        + decoded.samples[next_idx] as f64 * frac;
                    resampled.push(sample as i16);
                }
            }
        }

        Ok(DecodedAudio {
            samples: resampled,
            sample_rate: target_sample_rate,
            channels: decoded.channels,
            duration_ms: decoded.duration_ms,
        })
    }
}

impl Default for AudioDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_format() {
        // Minimal valid WAV header (44 bytes): silent 1-second 44100Hz mono 16-bit
        let wav_data = create_minimal_wav(44100, 1, 44100);
        let format = AudioDecoder::probe_format(&wav_data);
        assert!(format.is_ok());
        let fmt = format.unwrap();
        assert_eq!(fmt.sample_rate, 44100);
        assert_eq!(fmt.channels, 1);
    }

    #[test]
    fn test_decode_wav() {
        // Use a smaller WAV for more reliable decode
        let wav_data = create_minimal_wav(44100, 1, 4410);
        let decoded = AudioDecoder::decode(&wav_data);
        assert!(decoded.is_ok(), "Decode failed: {:?}", decoded.err());
        let audio = decoded.unwrap();
        assert_eq!(audio.sample_rate, 44100);
        assert_eq!(audio.channels, 1);
        assert!(!audio.samples.is_empty());
    }

    fn create_minimal_wav(sample_rate: u32, channels: u16, num_samples: u32) -> Vec<u8> {
        let data_size = num_samples * channels as u32 * 2; // 16-bit
        let file_size = 36 + data_size;
        let mut wav = Vec::with_capacity(44 + data_size as usize);

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * channels as u32 * 2;
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&(channels * 2).to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());

        // Silent PCM samples
        wav.resize(44 + data_size as usize, 0u8);

        wav
    }
}