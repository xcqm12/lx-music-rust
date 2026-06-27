package com.lx.music.lyric

import com.lx.music.core.model.LyricInfo
import com.lx.music.core.model.LyricLine
import com.lx.music.core.model.ParsedLyric
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.Json
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * 歌词存储库
 * 负责与 Rust 歌词模块通信
 */
@Singleton
class LyricRepository @Inject constructor() {
    
    private val json = Json { ignoreUnknownKeys = true }
    
    private val _currentLyric = MutableStateFlow(LyricState())
    val currentLyric: StateFlow<LyricState> = _currentLyric.asStateFlow()
    
    init {
        LyricBridge.initialize()
    }
    
    /**
     * 加载歌词
     */
    suspend fun loadLyric(lyricInfo: LyricInfo) {
        try {
            LyricBridge.loadLyric(lyricInfo)
            
            val parsedJson = LyricBridge.getCurrentLyric()
            val parsed = json.decodeFromString<ParsedLyric>(parsedJson)
            
            _currentLyric.value = _currentLyric.value.copy(
                parsedLyric = parsed,
                currentLineIndex = 0,
                currentLine = parsed.lines.firstOrNull()
            )
        } catch (e: Exception) {
            Timber.e(e, "Failed to load lyric")
        }
    }
    
    /**
     * 更新当前时间
     */
    suspend fun updateTime(timeMs: Long) {
        try {
            val lineIndex = LyricBridge.getCurrentLineIndex(timeMs)
            if (lineIndex >= 0) {
                val lineJson = LyricBridge.getCurrentLine(timeMs)
                val line = json.decodeFromString<LyricLine>(lineJson)
                
                _currentLyric.value = _currentLyric.value.copy(
                    currentLineIndex = lineIndex,
                    currentLine = line
                )
            }
        } catch (e: Exception) {
            Timber.e(e, "Failed to update lyric time")
        }
    }
    
    /**
     * 获取当前翻译
     */
    suspend fun getTranslation(timeMs: Long): String? {
        return try {
            LyricBridge.getTranslation(timeMs).takeIf { it.isNotEmpty() }
        } catch (e: Exception) {
            null
        }
    }
    
    /**
     * 获取当前罗马音
     */
    suspend fun getRomaji(timeMs: Long): String? {
        return try {
            LyricBridge.getRomaji(timeMs).takeIf { it.isNotEmpty() }
        } catch (e: Exception) {
            null
        }
    }
    
    /**
     * 清空歌词
     */
    fun clear() {
        LyricBridge.clear()
        _currentLyric.value = LyricState()
    }
    
    /**
     * 解析外部歌词文件
     */
    fun parseFile(content: String, format: String): ParsedLyric? {
        return try {
            val parsedJson = LyricBridge.parseFile(content, format)
            json.decodeFromString<ParsedLyric>(parsedJson)
        } catch (e: Exception) {
            Timber.e(e, "Failed to parse lyric file")
            null
        }
    }
}

// 数据类
data class LyricState(
    val parsedLyric: ParsedLyric? = null,
    val currentLineIndex: Int = -1,
    val currentLine: LyricLine? = null
)
