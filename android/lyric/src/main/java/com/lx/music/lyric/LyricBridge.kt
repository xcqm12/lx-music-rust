package com.lx.music.lyric

import com.lx.music.core.model.LyricInfo
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * 歌词桥接类
 * 封装了 Rust 歌词 FFI 接口
 */
object LyricBridge {
    
    init {
        System.loadLibrary("lx_music_core")
    }
    
    private val json = Json { ignoreUnknownKeys = true }
    
    /**
     * 初始化歌词管理器
     */
    @JvmStatic
    external fun initialize()
    
    /**
     * 加载歌词
     * @param lyricInfo 歌词信息
     */
    @JvmStatic
    fun loadLyric(lyricInfo: LyricInfo) {
        val lyricJson = json.encodeToString(lyricInfo)
        nativeLoadLyric(lyricJson)
    }
    
    @JvmStatic
    private external fun nativeLoadLyric(lyricInfoJson: String)
    
    /**
     * 获取当前歌词
     * @return JSON 格式的解析后歌词
     */
    @JvmStatic
    external fun getCurrentLyric(): String
    
    /**
     * 根据时间获取当前行
     * @param timeMs 时间（毫秒）
     * @return JSON 格式的歌词行
     */
    @JvmStatic
    external fun getCurrentLine(timeMs: Long): String
    
    /**
     * 根据时间获取当前行索引
     * @param timeMs 时间（毫秒）
     * @return 行索引，-1 表示未找到
     */
    @JvmStatic
    external fun getCurrentLineIndex(timeMs: Long): Int
    
    /**
     * 获取翻译
     * @param timeMs 时间（毫秒）
     * @return 翻译文本
     */
    @JvmStatic
    external fun getTranslation(timeMs: Long): String
    
    /**
     * 获取罗马音
     * @param timeMs 时间（毫秒）
     * @return 罗马音文本
     */
    @JvmStatic
    external fun getRomaji(timeMs: Long): String
    
    /**
     * 清空歌词
     */
    @JvmStatic
    external fun clear()
    
    /**
     * 解析外部歌词文件
     * @param content 歌词内容
     * @param format 格式（lrc, krc, yrc, qrc）
     * @return JSON 格式的解析后歌词
     */
    @JvmStatic
    external fun parseFile(content: String, format: String): String
}
