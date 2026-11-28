//! 真实音频输出：cpal 流 + 环形缓冲（桌面 WASAPI/CoreAudio；Android 走 oboe/AAudio）
//!
//! 音频回调线程只做三件事：从环形缓冲取样本、乘音量、写进设备——绝不阻塞。
//! 解码线程（audio_engine 的命令循环）负责解码、喂缓冲、按回调消费量推进进度。

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapCons, HeapProd, HeapRb, traits::{Split, Producer, Consumer, Observer}};

/// 简易日志（Android 上 println 进 logcat；桌面走 stderr）
fn log_or_print(msg: &str) {
    #[cfg(target_os = "android")]
    println!("[player/output] {msg}");
    #[cfg(not(target_os = "android"))]
    eprintln!("[player/output] {msg}");
}

/// 与 cpal 回调共享的运行时状态
struct Shared {
    prod: Mutex<HeapProd<f32>>,
    cons: Mutex<HeapCons<f32>>,
    volume: Mutex<f32>,            // 0..1（回调用，低竞争 Mutex 足够）
    played_samples: AtomicU64,     // 回调已消费样本数
    err_flag: AtomicBool,          // 回调错误标志
    err_count: AtomicU32,
    running: AtomicBool,
}

pub struct AudioOutput {
    shared: Arc<Shared>,
    _stream: Mutex<cpal::Stream>,
    sample_rate: u32,
    channels: u16,
}

impl AudioOutput {
    /// 以解码参数建流；设备不认该参数时回退默认配置（此时需调用方自行重采样率——
    /// 实验壳先按设备参数打开，绝大多数设备/模拟器支持任意采样率）。
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no audio output device".to_string())?;

        // 优先用解码参数建流（AAudio/大多数设备支持任意采样率）；失败再回退设备默认。
        // cpal 0.15 无 supports 检查 API，直接在 build_output_stream 时尝试。
        let requested: cpal::StreamConfig = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };
        let fallback: cpal::StreamConfig = device
            .default_output_config()
            .map_err(|e| format!("output config: {e}"))?
            .into();
        let actual_rate = fallback.sample_rate.0;
        let actual_ch = fallback.channels;

        let rb = HeapRb::<f32>::new((actual_rate as usize * actual_ch as usize / 2).max(4096));
        let (prod, cons) = rb.split();
        let shared = Arc::new(Shared {
            prod: Mutex::new(prod),
            cons: Mutex::new(cons),
            volume: Mutex::new(1.0),
            played_samples: AtomicU64::new(0),
            err_flag: AtomicBool::new(false),
            err_count: AtomicU32::new(0),
            running: AtomicBool::new(true),
        });

        let cb_shared = Arc::clone(&shared);
        let data_cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if !cb_shared.running.load(Ordering::Relaxed) {
                data.fill(0.0);
                return;
            }
            let vol = *cb_shared.volume.lock().unwrap_or_else(|p| p.into_inner());
            let mut cons = cb_shared.cons.lock().unwrap_or_else(|p| p.into_inner());
            for d in data.iter_mut() {
                *d = match cons.try_pop() {
                    Some(s) => s * vol,
                    None => {
                        // 欠载：填静音
                        0.0
                    }
                };
            }
            let consumed = data.len() as u64;
            cb_shared.played_samples.fetch_add(consumed, Ordering::Relaxed);
        };
        let err_shared = Arc::clone(&shared);
        let err_cb = move |err: cpal::StreamError| {
            err_shared.err_flag.store(true, Ordering::Relaxed);
            err_shared.err_count.fetch_add(1, Ordering::Relaxed);
        };

        // 先试解码参数（44.1k 等），设备不接受则回退默认参数重试（回调闭包需重建）
        let make_cbs = || {
            let cb_shared = Arc::clone(&shared);
            let data_cb = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if !cb_shared.running.load(Ordering::Relaxed) {
                    data.fill(0.0);
                    return;
                }
                let vol = *cb_shared.volume.lock().unwrap_or_else(|p| p.into_inner());
                let mut cons = cb_shared.cons.lock().unwrap_or_else(|p| p.into_inner());
                for d in data.iter_mut() {
                    *d = match cons.try_pop() {
                        Some(s) => s * vol,
                        None => 0.0,
                    };
                }
                let consumed = data.len() as u64;
                cb_shared.played_samples.fetch_add(consumed, Ordering::Relaxed);
            };
            let err_shared = Arc::clone(&shared);
            let err_cb = move |err| {
                err_shared.err_flag.store(true, Ordering::Relaxed);
                err_shared.err_count.fetch_add(1, Ordering::Relaxed);
            };
            (data_cb, err_cb)
        };

        let (stream, actual_rate, actual_ch) =
            match device.build_output_stream(&requested, make_cbs().0, make_cbs().1, None) {
                Ok(stream) => (
                    stream,
                    requested.sample_rate.0,
                    requested.channels,
                ),
                Err(e) => {
                    log_or_print(&format!(
                        "requested config rejected ({e}), falling back to device default"
                    ));
                    let (d0, d1) = make_cbs();
                    let stream = device
                        .build_output_stream(&fallback, d0, d1, None)
                        .map_err(|e| format!("build output stream (default cfg): {e}"))?;
                    (stream, fallback.sample_rate.0, fallback.channels)
                }
            };
        stream.play().map_err(|e| format!("start output stream: {e}"))?;

        Ok(Self {
            shared,
            _stream: Mutex::new(stream),
            sample_rate: actual_rate,
            channels: actual_ch,
            // resampled 标记留给调用方决定是否需要额外重采样（实验壳按设备参数直开）
        })
    }

    /// 写入解码样本（阻塞上限：缓冲半满即由调用方控制节奏）
    pub fn write(&self, samples: &[f32]) -> usize {
        let mut prod = self.shared.prod.lock().unwrap_or_else(|p| p.into_inner());
        prod.push_slice(samples)
    }

    /// 清空缓冲（暂停/seek 时丢弃未播放样本，保证进度精确）
    pub fn flush(&self) {
        let mut cons = self.shared.cons.lock().unwrap_or_else(|p| p.into_inner());
        while cons.try_pop().is_some() {}
    }

    /// 已实际播放秒数（由回调消费量推进——真实进度）
    pub fn played_seconds(&self) -> f64 {
        let n = self.shared.played_samples.load(Ordering::Relaxed);
        n as f64 / (self.sample_rate as f64 * self.channels as f64)
    }

    /// 重置播放计数（seek 后调用，position = seek 目标 + played_seconds）
    pub fn reset_played(&self) {
        self.shared.played_samples.store(0, Ordering::Relaxed);
    }

    /// 当前缓冲中待播样本数（推 buffered 进度用）
    pub fn queued_samples(&self) -> usize {
        let prod = self.shared.prod.lock().unwrap_or_else(|p| p.into_inner());
        prod.occupied_len()
    }

    pub fn capacity_samples(&self) -> usize {
        let prod = self.shared.prod.lock().unwrap_or_else(|p| p.into_inner());
        prod.capacity().get()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn set_volume(&self, v: f32) {
        let mut vol = self.shared.volume.lock().unwrap_or_else(|p| p.into_inner());
        *vol = v.clamp(0.0, 1.0);
    }

    /// 回调是否出错（引擎轮询后发 Error 事件）
    pub fn take_error(&self) -> Option<u32> {
        if self.shared.err_flag.swap(false, Ordering::Relaxed) {
            Some(self.shared.err_count.load(Ordering::Relaxed))
        } else {
            None
        }
    }
}
