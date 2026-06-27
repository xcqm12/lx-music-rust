use crate::types::{from_jstring, to_jstring, to_json, from_json};
use common::{MusicInfo, PlayMode, PlayerConfig};
use jni::objects::{JClass, JString};
use jni::signature::JavaType;
use jni::JNIEnv;
use player::Player;
use std::sync::Arc;
use tokio::runtime::Runtime;

lazy_static::lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
    static ref PLAYER: tokio::sync::Mutex<Option<Arc<Player>>> = tokio::sync::Mutex::new(None);
}

/// 初始化播放器
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_initialize(
    mut env: JNIEnv,
    _class: JClass,
    config_json: JString,
) {
    let config: PlayerConfig = match from_json(&mut env, &config_json) {
        Ok(c) => c,
        Err(_) => PlayerConfig::default(),
    };

    RUNTIME.block_on(async {
        let (player, _receiver) = match Player::new(config).await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to initialize player: {}", e);
                return;
            }
        };

        let mut guard = PLAYER.lock().await;
        *guard = Some(Arc::new(player));
    });
}

/// 播放
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_play(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.play().await {
                log::error!("Failed to play: {}", e);
            }
        }
    });
}

/// 暂停
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_pause(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.pause().await {
                log::error!("Failed to pause: {}", e);
            }
        }
    });
}

/// 停止
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_stop(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.stop().await {
                log::error!("Failed to stop: {}", e);
            }
        }
    });
}

/// 跳转到指定位置
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_seek(
    _env: JNIEnv,
    _class: JClass,
    position_ms: jni::sys::jlong,
) {
    let position = position_ms as f64 / 1000.0;
    
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.seek(position).await {
                log::error!("Failed to seek: {}", e);
            }
        }
    });
}

/// 设置音量
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_setVolume(
    _env: JNIEnv,
    _class: JClass,
    volume: jni::sys::jfloat,
) {
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.set_volume(volume).await {
                log::error!("Failed to set volume: {}", e);
            }
        }
    });
}

/// 获取音量
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_getVolume(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jfloat {
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            player.get_volume().await
        } else {
            1.0
        }
    })
}

/// 设置播放模式
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_setPlayMode(
    mut env: JNIEnv,
    _class: JClass,
    mode_json: JString,
) {
    let mode: PlayMode = match from_json(&mut env, &mode_json) {
        Ok(m) => m,
        Err(_) => return,
    };

    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.set_play_mode(mode).await {
                log::error!("Failed to set play mode: {}", e);
            }
        }
    });
}

/// 播放指定歌曲
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_playTrack(
    mut env: JNIEnv,
    _class: JClass,
    music_info_json: JString,
) {
    let music_info: MusicInfo = match from_json(&mut env, &music_info_json) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Failed to parse music info: {:?}", e);
            return;
        }
    };

    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.play_track(music_info).await {
                log::error!("Failed to play track: {}", e);
            }
        }
    });
}

/// 下一首
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_next(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.next().await {
                log::error!("Failed to play next: {}", e);
            }
        }
    });
}

/// 上一首
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_previous(
    _env: JNIEnv,
    _class: JClass,
) {
    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            if let Err(e) = player.previous().await {
                log::error!("Failed to play previous: {}", e);
            }
        }
    });
}

/// 添加到播放列表
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_addToPlaylist(
    mut env: JNIEnv,
    _class: JClass,
    music_info_json: JString,
) {
    let music_info: MusicInfo = match from_json(&mut env, &music_info_json) {
        Ok(m) => m,
        Err(_) => return,
    };

    RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            player.add_to_playlist(music_info).await;
        }
    });
}

/// 获取播放状态
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_getState(
    mut env: JNIEnv,
    _class: JClass,
) -> JString {
    let state = RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            player.get_state().await
        } else {
            common::PlayState::Idle
        }
    });

    to_jstring(&mut env, &format!("{:?}", state))
}

/// 获取播放进度
#[no_mangle]
pub extern "C" fn Java_com_lx_music_player_PlayerBridge_getProgress(
    mut env: JNIEnv,
    _class: JClass,
) -> JString {
    let progress = RUNTIME.block_on(async {
        if let Some(ref player) = *PLAYER.lock().await {
            player.get_progress().await
        } else {
            common::PlayProgress {
                position: 0.0,
                duration: 0.0,
                buffered: 0.0,
            }
        }
    });

    to_json(&mut env, &progress)
}
