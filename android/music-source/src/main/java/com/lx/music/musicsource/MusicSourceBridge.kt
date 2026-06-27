package com.lx.music.musicsource

import com.lx.music.core.model.LyricInfo
import com.lx.music.core.model.MusicInfo
import com.lx.music.core.model.MusicQuality
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * 音乐源桥接类
 * 封装了 Rust 音乐源 FFI 接口
 */
object MusicSourceBridge {
    
    init {
        System.loadLibrary("lx_music_core")
    }
    
    private val json = Json { ignoreUnknownKeys = true }
    
    /**
     * 初始化音乐源管理器
     */
    @JvmStatic
    external fun initialize()
    
    /**
     * 搜索音乐
     * @param sourceId 音乐源 ID (kw, kg, tx, wy, mg)
     * @param keyword 搜索关键词
     * @param page 页码（从1开始）
     * @param limit 每页数量
     * @return JSON 格式的搜索结果列表
     */
    @JvmStatic
    external fun search(
        sourceId: String,
        keyword: String,
        page: Int,
        limit: Int
    ): String
    
    /**
     * 获取音乐 URL
     * @param musicInfo 音乐信息
     * @param quality 音质
     * @return 音乐 URL
     */
    @JvmStatic
    fun getMusicUrl(musicInfo: MusicInfo, quality: MusicQuality): String {
        val musicJson = json.encodeToString(musicInfo)
        val qualityJson = json.encodeToString(quality)
        return nativeGetMusicUrl(musicJson, qualityJson)
    }
    
    @JvmStatic
    private external fun nativeGetMusicUrl(
        musicInfoJson: String,
        qualityJson: String
    ): String
    
    /**
     * 获取歌词
     * @param musicInfo 音乐信息
     * @return JSON 格式的歌词信息
     */
    @JvmStatic
    fun getLyric(musicInfo: MusicInfo): String {
        val musicJson = json.encodeToString(musicInfo)
        return nativeGetLyric(musicJson)
    }
    
    @JvmStatic
    private external fun nativeGetLyric(musicInfoJson: String): String
    
    /**
     * 跨源查找音乐
     * @param musicInfo 音乐信息
     * @return JSON 格式的音乐列表
     */
    @JvmStatic
    fun findMusicCrossSource(musicInfo: MusicInfo): String {
        val musicJson = json.encodeToString(musicInfo)
        return nativeFindMusicCrossSource(musicJson)
    }
    
    @JvmStatic
    private external fun nativeFindMusicCrossSource(musicInfoJson: String): String
    
    /**
     * 获取可用音乐源列表
     * @return JSON 格式的源 ID 列表
     */
    @JvmStatic
    external fun getAvailableSources(): String
}
