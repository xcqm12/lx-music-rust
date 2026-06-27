use crate::types::{to_jstring, to_json, from_json};
use common::{MusicInfo, MusicQuality, MusicSource};
use jni::objects::{JClass, JString};
use jni::JNIEnv;
use music_source::{MusicSourceManager, MusicSourceProvider};
use std::sync::Arc;
use tokio::runtime::Runtime;

lazy_static::lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
    static ref SOURCE_MANAGER: tokio::sync::Mutex<Option<Arc<MusicSourceManager>>> = 
        tokio::sync::Mutex::new(None);
}

/// 初始化音乐源管理器
#[no_mangle]
pub extern "C" fn Java_com_lx_music_musicsource_MusicSourceBridge_initialize(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        let manager = Arc::new(MusicSourceManager::new());
        let mut guard = SOURCE_MANAGER.lock().await;
        *guard = Some(manager);
    });
}

/// 搜索音乐
#[no_mangle]
pub extern "C" fn Java_com_lx_music_musicsource_MusicSourceBridge_search(
    mut env: JNIEnv,
    _class: JClass,
    source_id: JString,
    keyword: JString,
    page: jni::sys::jint,
    limit: jni::sys::jint,
) -> JString {
    let source_str = match env.get_string(&source_id) {
        Ok(s) => s.to_string_lossy().to_string(),
        Err(_) => return to_jstring(&mut env, "[]"),
    };

    let keyword_str = match env.get_string(&keyword) {
        Ok(s) => s.to_string_lossy().to_string(),
        Err(_) => return to_jstring(&mut env, "[]"),
    };

    let source = match MusicSource::try_from(source_str.as_str()) {
        Ok(s) => s,
        Err(_) => return to_jstring(&mut env, "[]"),
    };

    let results = RUNTIME.block_on(async {
        if let Some(ref manager) = *SOURCE_MANAGER.lock().await {
            match manager.search(source, &keyword_str, page as u32, limit as u32).await {
                Ok(results) => results,
                Err(e) => {
                    log::error!("Search failed: {}", e);
                    vec![]
                }
            }
        } else {
            vec![]
        }
    });

    to_json(&mut env, &results)
}

/// 获取音乐 URL
#[no_mangle]
pub extern "C" fn Java_com_lx_music_musicsource_MusicSourceBridge_getMusicUrl(
    mut env: JNIEnv,
    _class: JClass,
    music_info_json: JString,
    quality_json: JString,
) -> JString {
    let music_info: MusicInfo = match from_json(&mut env, &music_info_json) {
        Ok(m) => m,
        Err(_) => return to_jstring(&mut env, ""),
    };

    let quality: MusicQuality = match from_json(&mut env, &quality_json) {
        Ok(q) => q,
        Err(_) => MusicQuality::Hq,
    };

    let url = RUNTIME.block_on(async {
        if let Some(ref manager) = *SOURCE_MANAGER.lock().await {
            match manager.get_music_url(&music_info, quality).await {
                Ok(u) => u,
                Err(e) => {
                    log::error!("Failed to get music URL: {}", e);
                    String::new()
                }
            }
        } else {
            String::new()
        }
    });

    to_jstring(&mut env, &url)
}

/// 获取歌词
#[no_mangle]
pub extern "C" fn Java_com_lx_music_musicsource_MusicSourceBridge_getLyric(
    mut env: JNIEnv,
    _class: JClass,
    music_info_json: JString,
) -> JString {
    let music_info: MusicInfo = match from_json(&mut env, &music_info_json) {
        Ok(m) => m,
        Err(_) => return to_jstring(&mut env, "{}"),
    };

    let lyric_info = RUNTIME.block_on(async {
        if let Some(ref manager) = *SOURCE_MANAGER.lock().await {
            match manager.get_lyric(&music_info).await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("Failed to get lyric: {}", e);
                    common::LyricInfo {
                        lyric: String::new(),
                        tlyric: None,
                        rlyric: None,
                        lxlyric: None,
                    }
                }
            }
        } else {
            common::LyricInfo {
                lyric: String::new(),
                tlyric: None,
                rlyric: None,
                lxlyric: None,
            }
        }
    });

    to_json(&mut env, &lyric_info)
}

/// 跨源查找音乐
#[no_mangle]
pub extern "C" fn Java_com_lx_music_musicsource_MusicSourceBridge_findMusicCrossSource(
    mut env: JNIEnv,
    _class: JClass,
    music_info_json: JString,
) -> JString {
    let music_info: MusicInfo = match from_json(&mut env, &music_info_json) {
        Ok(m) => m,
        Err(_) => return to_jstring(&mut env, "[]"),
    };

    let results = RUNTIME.block_on(async {
        if let Some(ref manager) = *SOURCE_MANAGER.lock().await {
            match manager.find_music_cross_source(&music_info).await {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Cross source search failed: {}", e);
                    vec![]
                }
            }
        } else {
            vec![]
        }
    });

    to_json(&mut env, &results)
}

/// 获取可用音乐源
#[no_mangle]
pub extern "C" fn Java_com_lx_music_musicsource_MusicSourceBridge_getAvailableSources(
    mut env: JNIEnv,
    _class: JClass,
) -> JString {
    let sources: Vec<String> = vec![
        "kw".to_string(), // 酷我
        "kg".to_string(), // 酷狗
        "tx".to_string(), // QQ音乐
        "wy".to_string(), // 网易云
        "mg".to_string(), // 咪咕
    ];

    to_json(&mut env, &sources)
}
