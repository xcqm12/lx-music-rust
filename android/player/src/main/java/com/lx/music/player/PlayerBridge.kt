package com.lx.music.player

import com.lx.music.core.model.MusicInfo
import com.lx.music.core.model.PlayMode
import com.lx.music.core.model.PlayerConfig
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * 播放器桥接类
 * 封装了 Rust 播放器 FFI 接口
 */
object PlayerBridge {
    
    init {
        System.loadLibrary("lx_music_core")
    }
    
    private val json = Json { ignoreUnknownKeys = true }
    
    /**
     * 初始化播放器
     */
    @JvmStatic
    fun initialize(config: PlayerConfig) {
        val configJson = json.encodeToString(config)
        nativeInitialize(configJson)
    }
    
    @JvmStatic
    private external fun nativeInitialize(configJson: String)
    
    /**
     * 播放
     */
    @JvmStatic
    external fun play()
    
    /**
     * 暂停
     */
    @JvmStatic
    external fun pause()
    
    /**
     * 停止
     */
    @JvmStatic
    external fun stop()
    
    /**
     * 跳转到指定位置
     * @param positionMs 位置（毫秒）
     */
    @JvmStatic
    external fun seek(positionMs: Long)
    
    /**
     * 设置音量
     * @param volume 音量（0.0 - 1.0）
     */
    @JvmStatic
    external fun setVolume(volume: Float)
    
    /**
     * 获取音量
     */
    @JvmStatic
    external fun getVolume(): Float
    
    /**
     * 设置播放模式
     */
    @JvmStatic
    fun setPlayMode(mode: PlayMode) {
        val modeJson = json.encodeToString(mode)
        nativeSetPlayMode(modeJson)
    }
    
    @JvmStatic
    private external fun nativeSetPlayMode(modeJson: String)
    
    /**
     * 播放指定歌曲
     */
    @JvmStatic
    fun playTrack(musicInfo: MusicInfo) {
        val musicJson = json.encodeToString(musicInfo)
        nativePlayTrack(musicJson)
    }
    
    @JvmStatic
    private external fun nativePlayTrack(musicInfoJson: String)
    
    /**
     * 下一首
     */
    @JvmStatic
    external fun next()
    
    /**
     * 上一首
     */
    @JvmStatic
    external fun previous()
    
    /**
     * 添加到播放列表
     */
    @JvmStatic
    fun addToPlaylist(musicInfo: MusicInfo) {
        val musicJson = json.encodeToString(musicInfo)
        nativeAddToPlaylist(musicJson)
    }
    
    @JvmStatic
    private external fun nativeAddToPlaylist(musicInfoJson: String)
    
    /**
     * 获取播放状态
     */
    @JvmStatic
    external fun getState(): String
    
    /**
     * 获取播放进度
     */
    @JvmStatic
    external fun getProgress(): String
}
