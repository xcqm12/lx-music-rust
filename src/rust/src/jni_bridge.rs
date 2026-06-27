//! JNI Bridge Module
//! 
//! Provides JNI bindings to expose Rust functions to Android Java/Kotlin code.
//! This is the bridge between TurboModule (C++) and Rust core library.

use jni::{
    objects::{JClass, JString, JObject, JByteArray, JShortArray},
    sys::{jstring, jobjectArray, jboolean, jfloat, jlong, jint},
    JNIEnv,
};
use base64::Engine;
use std::sync::{Mutex, Once};
use crate::player::PlayerEngine;
use crate::lyric::LyricEngine;
use crate::sources::{get_source_manager, kw, kg, mg};
use crate::audio_decoder::AudioDecoder;

#[cfg(feature = "js-engine")]
use crate::js_engine::JsEngine;

static ENGINE_INIT: Once = Once::new();

#[cfg(feature = "js-engine")]
static JS_ENGINE: Mutex<Option<JsEngine>> = Mutex::new(None);

static PLAYER_ENGINE: Mutex<Option<PlayerEngine>> = Mutex::new(None);
static LYRIC_ENGINE: Mutex<Option<LyricEngine>> = Mutex::new(None);

/// Initialize all engines
fn init_engines() {
    ENGINE_INIT.call_once(|| {
        #[cfg(feature = "js-engine")]
        {
            if let Ok(engine) = JsEngine::new() {
                *JS_ENGINE.lock().unwrap() = Some(engine);
            }
        }
        
        *PLAYER_ENGINE.lock().unwrap() = Some(PlayerEngine::new());
        *LYRIC_ENGINE.lock().unwrap() = Some(LyricEngine::new());

        // Register native Rust music sources
        if let Ok(mut mgr) = get_source_manager().write() {
            mgr.register_native(Box::new(kw::KwSource::new()));
            mgr.register_native(Box::new(kg::KgSource::new()));
            mgr.register_native(Box::new(mg::MgSource::new()));
            // tx, wy still use JS engine
            mgr.register_js("tx", "QQ音乐");
            mgr.register_js("wy", "网易音乐");
        }
    });
}

/// Get player engine guard
fn get_player() -> std::sync::MutexGuard<'static, Option<PlayerEngine>> {
    init_engines();
    PLAYER_ENGINE.lock().unwrap()
}

/// Get lyric engine guard
fn get_lyric() -> std::sync::MutexGuard<'static, Option<LyricEngine>> {
    init_engines();
    LYRIC_ENGINE.lock().unwrap()
}

#[cfg(feature = "js-engine")]
fn get_js_engine() -> std::sync::MutexGuard<'static, Option<JsEngine>> {
    init_engines();
    JS_ENGINE.lock().unwrap()
}

// ============================================================================
// Initialization
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_initEngine(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    init_engines();
    true as jboolean
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_isInitialized(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let player_ok = PLAYER_ENGINE.lock().map(|g| g.is_some()).unwrap_or(false);
    let lyric_ok = LYRIC_ENGINE.lock().map(|g| g.is_some()).unwrap_or(false);
    (player_ok && lyric_ok) as jboolean
}

// ============================================================================
// Music Source Functions (JNI)
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_loadSource(
    env: JNIEnv,
    _class: JClass,
    source_id: JString,
    source_name: JString,
    source_code: JString,
) -> jstring {
    #[cfg(feature = "js-engine")]
    {
        let id: String = env.get_string(&source_id).unwrap().into();
        let name: String = env.get_string(&source_name).unwrap().into();
        let code: String = env.get_string(&source_code).unwrap().into();

        let result = match get_js_engine().as_mut() {
            Some(engine) => engine.load_source(&id, &name, &code).map(|_| "OK"),
            None => Err(crate::js_engine::JsError::EngineNotInitialized),
        };

        match result {
            Ok(msg) => env.new_string(msg).unwrap().into_raw(),
            Err(e) => env.new_string(format!("Error: {}", e)).unwrap().into_raw(),
        }
    }

    #[cfg(not(feature = "js-engine"))]
    {
        let _ = (&source_id, &source_name, &source_code);
        env.new_string("JS engine not available").unwrap().into_raw()
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_searchMusic(
    env: JNIEnv,
    _class: JClass,
    source_id: JString,
    keyword: JString,
) -> jstring {
    #[cfg(feature = "js-engine")]
    {
        let id: String = env.get_string(&source_id).unwrap().into();
        let kw: String = env.get_string(&keyword).unwrap().into();

        let result = match get_js_engine().as_mut() {
            Some(engine) => engine.search(&id, &kw),
            None => Err(crate::js_engine::JsError::EngineNotInitialized),
        };

        match result {
            Ok(results) => {
                let json = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
                env.new_string(json).unwrap().into_raw()
            }
            Err(e) => env.new_string(format!("Error: {}", e)).unwrap().into_raw(),
        }
    }

    #[cfg(not(feature = "js-engine"))]
    {
        let _ = (&source_id, &keyword);
        env.new_string("[]").unwrap().into_raw()
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_getMusicUrl(
    env: JNIEnv,
    _class: JClass,
    source_id: JString,
    music_id: JString,
    quality: JString,
) -> jstring {
    #[cfg(feature = "js-engine")]
    {
        let id: String = env.get_string(&source_id).unwrap().into();
        let mid: String = env.get_string(&music_id).unwrap().into();
        let q: String = env.get_string(&quality).unwrap().into();

        let result = match get_js_engine().as_mut() {
            Some(engine) => engine.get_music_url(&id, &mid, &q),
            None => Err(crate::js_engine::JsError::EngineNotInitialized),
        };

        match result {
            Ok(url) => {
                let json = serde_json::to_string(&url).unwrap_or_else(|_| "null".to_string());
                env.new_string(json).unwrap().into_raw()
            }
            Err(e) => env.new_string(format!("Error: {}", e)).unwrap().into_raw(),
        }
    }

    #[cfg(not(feature = "js-engine"))]
    {
        let _ = (&source_id, &music_id, &quality);
        env.new_string("null").unwrap().into_raw()
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_getLyric(
    env: JNIEnv,
    _class: JClass,
    source_id: JString,
    music_id: JString,
) -> jstring {
    #[cfg(feature = "js-engine")]
    {
        let id: String = env.get_string(&source_id).unwrap().into();
        let mid: String = env.get_string(&music_id).unwrap().into();

        let result = match get_js_engine().as_mut() {
            Some(engine) => engine.get_lyric(&id, &mid),
            None => Err(crate::js_engine::JsError::EngineNotInitialized),
        };

        match result {
            Ok(lyric) => {
                let json = serde_json::to_string(&lyric).unwrap_or_else(|_| "null".to_string());
                env.new_string(json).unwrap().into_raw()
            }
            Err(e) => env.new_string(format!("Error: {}", e)).unwrap().into_raw(),
        }
    }

    #[cfg(not(feature = "js-engine"))]
    {
        let _ = (&source_id, &music_id);
        env.new_string("null").unwrap().into_raw()
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_getPic(
    env: JNIEnv,
    _class: JClass,
    source_id: JString,
    music_id: JString,
) -> jstring {
    #[cfg(feature = "js-engine")]
    {
        let id: String = env.get_string(&source_id).unwrap().into();
        let mid: String = env.get_string(&music_id).unwrap().into();

        let result = match get_js_engine().as_mut() {
            Some(engine) => engine.get_pic(&id, &mid),
            None => Err(crate::js_engine::JsError::EngineNotInitialized),
        };

        match result {
            Ok(pic) => {
                let json = serde_json::to_string(&pic).unwrap_or_else(|_| "null".to_string());
                env.new_string(json).unwrap().into_raw()
            }
            Err(e) => env.new_string(format!("Error: {}", e)).unwrap().into_raw(),
        }
    }

    #[cfg(not(feature = "js-engine"))]
    {
        let _ = (&source_id, &music_id);
        env.new_string("null").unwrap().into_raw()
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_getSources(
    mut env: JNIEnv,
    _class: JClass,
) -> jobjectArray {
    #[cfg(feature = "js-engine")]
    {
        let sources = match get_js_engine().as_mut() {
            Some(engine) => engine.get_sources(),
            None => Vec::new(),
        };

        let java_string_class = env.find_class("java/lang/String").unwrap();
        let array = env.new_object_array(
            (sources.len() * 2) as i32,
            java_string_class,
            JObject::null(),
        ).unwrap();

        for (i, (id, name)) in sources.iter().enumerate() {
            let id_jstring = env.new_string(id).unwrap();
            let name_jstring = env.new_string(name).unwrap();
            env.set_object_array_element(array, (i * 2) as i32, id_jstring).unwrap();
            env.set_object_array_element(array, (i * 2 + 1) as i32, name_jstring).unwrap();
        }

        array.into_raw()
    }

    #[cfg(not(feature = "js-engine"))]
    {
        let java_string_class = env.find_class("java/lang/String").unwrap();
        env.new_object_array(0, java_string_class, JObject::null()).unwrap().into_raw()
    }
}

// ============================================================================
// Player Functions (JNI)
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerPlay(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(player) = get_player().as_mut() {
        player.play();
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerPause(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(player) = get_player().as_mut() {
        player.pause();
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerStop(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(player) = get_player().as_mut() {
        player.stop();
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerToggle(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(player) = get_player().as_mut() {
        player.toggle_play();
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerNext(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(player) = get_player().as_mut() {
        player.next();
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerPrev(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(player) = get_player().as_mut() {
        player.prev();
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerSeek(
    _env: JNIEnv,
    _class: JClass,
    time: jlong,
) {
    if let Some(player) = get_player().as_mut() {
        player.seek(time as u64);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerSetVolume(
    _env: JNIEnv,
    _class: JClass,
    volume: jfloat,
) {
    if let Some(player) = get_player().as_mut() {
        player.set_volume(volume as f32);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerSetPlaybackRate(
    _env: JNIEnv,
    _class: JClass,
    rate: jfloat,
) {
    if let Some(player) = get_player().as_mut() {
        player.set_playback_rate(rate as f32);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerSetPlayMode(
    _env: JNIEnv,
    _class: JClass,
    mode: jint,
) {
    if let Some(player) = get_player().as_mut() {
        player.set_play_mode_int(mode as i32);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerSetPlaylist(
    mut env: JNIEnv,
    _class: JClass,
    playlist_json: JString,
) -> jstring {
    let playlist_str: String = env.get_string(&playlist_json).unwrap().into();
    
    if let Some(player) = get_player().as_mut() {
        match player.set_playlist_json(&playlist_str) {
            Ok(_) => return env.new_string("OK").unwrap().into_raw(),
            Err(e) => return env.new_string(format!("Error: {}", e)).unwrap().into_raw(),
        }
    }
    
    env.new_string("Error: Player not initialized").unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerPlayAtIndex(
    _env: JNIEnv,
    _class: JClass,
    index: jint,
) {
    if let Some(player) = get_player().as_mut() {
        player.play_at_index(index as usize);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerGetState(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let state_json = if let Some(player) = get_player().as_mut() {
        player.get_state_json()
    } else {
        "{}".to_string()
    };

    env.new_string(state_json).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerAddToPlaylist(
    mut env: JNIEnv,
    _class: JClass,
    music_json: JString,
) -> jstring {
    let music_str: String = env.get_string(&music_json).unwrap().into();

    if let Some(player) = get_player().as_mut() {
        match player.add_to_playlist_json(&music_str) {
            Ok(_) => env.new_string("OK").unwrap().into_raw(),
            Err(e) => env.new_string(format!("Error: {}", e)).unwrap().into_raw(),
        }
    } else {
        env.new_string("Error: Player not initialized").unwrap().into_raw()
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerRemoveFromPlaylist(
    _env: JNIEnv,
    _class: JClass,
    index: jint,
) {
    if let Some(player) = get_player().as_mut() {
        player.remove_from_playlist(index as usize);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_playerClearPlaylist(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(player) = get_player().as_mut() {
        player.clear_playlist();
    }
}

// ============================================================================
// Lyric Functions (JNI)
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricSetLyric(
    mut env: JNIEnv,
    _class: JClass,
    lyric: JString,
    translation: JString,
) {
    let lyric_str: String = env.get_string(&lyric).unwrap().into();
    let translation_str: String = env.get_string(&translation).unwrap().into();

    if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.set_lyric(&lyric_str, &translation_str);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricGetCurrentLine(
    env: JNIEnv,
    _class: JClass,
    time_ms: jlong,
) -> jstring {
    let line_json = if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.get_current_line_json(time_ms as u64)
    } else {
        "null".to_string()
    };

    env.new_string(line_json).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricGetLineIndex(
    _env: JNIEnv,
    _class: JClass,
    time_ms: jlong,
) -> jint {
    if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.get_line_index(time_ms as u64)
    } else {
        -1
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricGetLines(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let lines_json = if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.get_lines_json()
    } else {
        "[]".to_string()
    };

    env.new_string(lines_json).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricSetPlaybackRate(
    _env: JNIEnv,
    _class: JClass,
    rate: jfloat,
) {
    if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.set_playback_rate(rate as f32);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricToggleTranslation(
    _env: JNIEnv,
    _class: JClass,
    show: jboolean,
) {
    if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.toggle_translation(show != 0);
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricIsShowTranslation(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.is_show_translation() as jboolean
    } else {
        false as jboolean
    }
}

#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_lyricClear(
    _env: JNIEnv,
    _class: JClass,
) {
    if let Some(lyric_engine) = get_lyric().as_mut() {
        lyric_engine.clear();
    }
}

// ============================================================================
// Audio Decoder Functions (JNI) - Symphonia
// ============================================================================

/// Probe audio data and return format info as JSON
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioProbeFormat(
    env: JNIEnv,
    _class: JClass,
    data: JByteArray,
) -> jstring {
    let data_len = env.get_array_length(&data).unwrap_or(0) as usize;
    if data_len == 0 {
        return env.new_string("{}").unwrap().into_raw();
    }

    let mut buf = vec![0i8; data_len];
    env.get_byte_array_region(&data, 0, &mut buf).unwrap();

    let buf_u8: Vec<u8> = buf.iter().map(|&b| b as u8).collect();
    let result = match AudioDecoder::probe_format(&buf_u8) {
        Ok(format) => serde_json::json!({
            "codec": format.codec,
            "sampleRate": format.sample_rate,
            "channels": format.channels,
            "duration": format.duration_ms,
            "bitrate": format.bitrate_kbps,
            "totalFrames": format.total_frames,
        }).to_string(),
        Err(e) => serde_json::json!({
            "error": e
        }).to_string(),
    };

    env.new_string(result).unwrap().into_raw()
}

/// Decode audio data and return PCM samples as i16 byte array
/// Returns JSON with metadata + base64-encoded PCM bytes
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioDecode(
    env: JNIEnv,
    _class: JClass,
    data: JByteArray,
) -> jstring {
    let data_len = env.get_array_length(&data).unwrap_or(0) as usize;
    if data_len == 0 {
        return env.new_string("{}").unwrap().into_raw();
    }

    let mut buf = vec![0i8; data_len];
    env.get_byte_array_region(&data, 0, &mut buf).unwrap();

    let buf_u8: Vec<u8> = buf.iter().map(|&b| b as u8).collect();
    let result = match AudioDecoder::decode(&buf_u8) {
        Ok(decoded) => {
            let pcm_bytes: Vec<u8> = decoded.samples.iter()
                .flat_map(|s| s.to_le_bytes())
                .collect();
            let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);

            serde_json::json!({
                "sampleRate": decoded.sample_rate,
                "channels": decoded.channels,
                "duration": decoded.duration_ms,
                "pcmBase64": pcm_base64,
                "pcmSize": decoded.samples.len(),
            }).to_string()
        }
        Err(e) => serde_json::json!({
            "error": e
        }).to_string(),
    };

    env.new_string(result).unwrap().into_raw()
}

/// Decode audio data and resample to target sample rate
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioDecodeResampled(
    env: JNIEnv,
    _class: JClass,
    data: JByteArray,
    target_sample_rate: jint,
) -> jstring {
    let data_len = env.get_array_length(&data).unwrap_or(0) as usize;
    if data_len == 0 {
        return env.new_string("{}").unwrap().into_raw();
    }

    let mut buf = vec![0i8; data_len];
    env.get_byte_array_region(&data, 0, &mut buf).unwrap();

    let buf_u8: Vec<u8> = buf.iter().map(|&b| b as u8).collect();
    let result = match AudioDecoder::decode_resampled(&buf_u8, target_sample_rate as u32) {
        Ok(decoded) => {
            let pcm_bytes: Vec<u8> = decoded.samples.iter()
                .flat_map(|s| s.to_le_bytes())
                .collect();
            let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);

            serde_json::json!({
                "sampleRate": decoded.sample_rate,
                "channels": decoded.channels,
                "duration": decoded.duration_ms,
                "pcmBase64": pcm_base64,
                "pcmSize": decoded.samples.len(),
            }).to_string()
        }
        Err(e) => serde_json::json!({
            "error": e
        }).to_string(),
    };

    env.new_string(result).unwrap().into_raw()
}

// ============================================================================
// Audio Output Functions (JNI)
// ============================================================================

/// Queue PCM buffer for playback
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioQueueBuffer(
    env: JNIEnv,
    _class: JClass,
    samples: JShortArray,
    sample_rate: jint,
    channels: jint,
) {
    if let Some(player) = get_player().as_mut() {
        let len = env.get_array_length(&samples).unwrap_or(0) as usize;
        if len == 0 {
            return;
        }
        let mut buf = vec![0i16; len];
        env.get_short_array_region(&samples, 0, &mut buf).unwrap();
        player.queue_audio_buffer(buf, sample_rate as u32, channels as u16);
    }
}

/// Dequeue next PCM buffer for Android AudioTrack
/// Returns JSON with base64 PCM data or null
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioDequeueBuffer(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    if let Some(player) = get_player().as_mut() {
        match player.dequeue_audio_buffer() {
            Some(buffer) => {
                let pcm_bytes: Vec<u8> = buffer.samples.iter()
                    .flat_map(|s| s.to_le_bytes())
                    .collect();
                let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);
                let result = serde_json::json!({
                    "sampleRate": buffer.sample_rate,
                    "channels": buffer.channels,
                    "pcmBase64": pcm_base64,
                    "pcmSize": buffer.samples.len(),
                }).to_string();
                env.new_string(result).unwrap().into_raw()
            }
            None => env.new_string("null").unwrap().into_raw(),
        }
    } else {
        env.new_string("null").unwrap().into_raw()
    }
}

/// Get audio buffer count
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioBufferCount(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    if let Some(player) = get_player().as_mut() {
        player.audio_buffer_count() as jint
    } else {
        0
    }
}

/// Get audio buffer size recommendation in bytes
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioBufferSizeBytes(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    if let Some(player) = get_player().as_mut() {
        player.audio_buffer_size_bytes() as jint
    } else {
        0
    }
}

/// Check if audio output is playing
#[no_mangle]
pub extern "system" fn Java_cn_toside_music_mobile_RustBridge_audioIsPlaying(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    if let Some(player) = get_player().as_mut() {
        player.is_audio_playing() as jboolean
    } else {
        false as jboolean
    }
}