use crate::types::{from_json, to_json, to_jstring};
use common::LyricInfo;
use jni::objects::{JClass, JString};
use jni::JNIEnv;
use lyric::{LyricManager, LyricEvent, ParsedLyric};
use std::sync::Arc;
use tokio::runtime::Runtime;

lazy_static::lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
    static ref LYRIC_MANAGER: tokio::sync::Mutex<Option<Arc<LyricManager>>> = 
        tokio::sync::Mutex::new(None);
}

/// 初始化歌词管理器
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_initialize(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        // 创建事件通道
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::channel(100);
        
        let manager = Arc::new(LyricManager::new(event_sender));
        
        // 启动事件处理任务
        RUNTIME.spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                handle_lyric_event(event);
            }
        });
        
        let mut guard = LYRIC_MANAGER.lock().await;
        *guard = Some(manager);
    });
}

/// 加载歌词
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_loadLyric(
    mut env: JNIEnv,
    _class: JClass,
    lyric_info_json: JString,
) {
    let lyric_info: LyricInfo = match from_json(&mut env, &lyric_info_json) {
        Ok(l) => l,
        Err(_) => return,
    };

    RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            if let Err(e) = manager.load_lyric(lyric_info).await {
                log::error!("Failed to load lyric: {}", e);
            }
        }
    });
}

/// 获取当前歌词
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_getCurrentLyric(
    mut env: JNIEnv,
    _class: JClass,
) -> JString {
    let lyric = RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            manager.get_current_lyric().await
        } else {
            None
        }
    });

    match lyric {
        Some(l) => to_json(&mut env, &l),
        None => to_jstring(&mut env, "{}"),
    }
}

/// 根据时间获取当前行
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_getCurrentLine(
    mut env: JNIEnv,
    _class: JClass,
    time_ms: jni::sys::jlong,
) -> JString {
    let line = RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            manager.get_current_line(time_ms as f64).await
        } else {
            None
        }
    });

    match line {
        Some(l) => to_json(&mut env, &l),
        None => to_jstring(&mut env, "{}"),
    }
}

/// 根据时间获取当前行索引
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_getCurrentLineIndex(
    _env: JNIEnv,
    _class: JClass,
    time_ms: jni::sys::jlong,
) -> jni::sys::jint {
    RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            manager.get_current_line_index(time_ms as f64).await
                .map(|i| i as jni::sys::jint)
                .unwrap_or(-1)
        } else {
            -1
        }
    })
}

/// 获取翻译
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_getTranslation(
    mut env: JNIEnv,
    _class: JClass,
    time_ms: jni::sys::jlong,
) -> JString {
    let translation = RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            manager.get_translation(time_ms as f64).await
        } else {
            None
        }
    });

    match translation {
        Some(t) => to_jstring(&mut env, &t),
        None => to_jstring(&mut env, ""),
    }
}

/// 获取罗马音
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_getRomaji(
    mut env: JNIEnv,
    _class: JClass,
    time_ms: jni::sys::jlong,
) -> JString {
    let romaji = RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            manager.get_romaji(time_ms as f64).await
        } else {
            None
        }
    });

    match romaji {
        Some(r) => to_jstring(&mut env, &r),
        None => to_jstring(&mut env, ""),
    }
}

/// 清空歌词
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_clear(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            manager.clear().await;
        }
    });
}

/// 解析外部歌词文件
#[no_mangle]
pub extern "C" fn Java_com_lx_music_lyric_LyricBridge_parseFile(
    mut env: JNIEnv,
    _class: JClass,
    content: JString,
    format: JString,
) -> JString {
    let content_str = match env.get_string(&content) {
        Ok(s) => s.to_string_lossy().to_string(),
        Err(_) => return to_jstring(&mut env, "{}"),
    };

    let format_str = match env.get_string(&format) {
        Ok(s) => s.to_string_lossy().to_string(),
        Err(_) => return to_jstring(&mut env, "{}"),
    };

    let lyric_format = match format_str.as_str() {
        "lrc" => lyric::LyricFormat::Lrc,
        "krc" => lyric::LyricFormat::Krc,
        "yrc" => lyric::LyricFormat::Yrc,
        "qrc" => lyric::LyricFormat::Qrc,
        _ => lyric::LyricFormat::Auto,
    };

    let lyric = RUNTIME.block_on(async {
        if let Some(ref manager) = *LYRIC_MANAGER.lock().await {
            manager.parse_file(&content_str, lyric_format).ok()
        } else {
            None
        }
    });

    match lyric {
        Some(l) => to_json(&mut env, &l),
        None => to_jstring(&mut env, "{}"),
    }
}

/// 处理歌词事件
fn handle_lyric_event(event: LyricEvent) {
    match event {
        LyricEvent::Loaded(lyric) => {
            log::info!("Lyric loaded with {} lines", lyric.lines.len());
        }
        LyricEvent::LineChanged(index, line) => {
            log::debug!("Lyric line changed to {}: {}", index, line.text);
        }
        LyricEvent::WordChanged(line_idx, word_idx, time) => {
            log::debug!("Lyric word changed at line {} word {} time {}", line_idx, word_idx, time);
        }
        LyricEvent::Error(msg) => {
            log::error!("Lyric error: {}", msg);
        }
    }
}
