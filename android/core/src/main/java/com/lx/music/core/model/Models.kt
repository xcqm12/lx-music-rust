package com.lx.music.core.model

import kotlinx.serialization.Serializable

/**
 * 音乐信息
 */
@Serializable
data class MusicInfo(
    val id: String,
    val name: String,
    val singer: List<String>,
    val albumName: String,
    val interval: Int, // 时长（秒）
    val source: MusicSource,
    val quality: Map<MusicQuality, String>,
    val picUrl: String? = null
)

/**
 * 音乐来源
 */
@Serializable
enum class MusicSource {
    Kw,  // 酷我
    Kg,  // 酷狗
    Tx,  // QQ音乐
    Wy,  // 网易云
    Mg,  // 咪咕
    Local
}

/**
 * 音质类型
 */
@Serializable
enum class MusicQuality {
    Lq,   // 低质量 (128k)
    Mq,   // 中等质量 (192k)
    Hq,   // 高质量 (320k)
    Sq,   // 超高质量 (FLAC)
    Hires // Hi-Res
}

/**
 * 播放状态
 */
@Serializable
enum class PlayState {
    Idle,
    Playing,
    Paused,
    Stopped,
    Buffering,
    Error
}

/**
 * 播放模式
 */
@Serializable
enum class PlayMode {
    Order,     // 顺序播放
    Loop,      // 列表循环
    Random,    // 随机播放
    Single     // 单曲循环
}

/**
 * 播放进度
 */
@Serializable
data class PlayProgress(
    val position: Double,  // 当前位置（秒）
    val duration: Double,  // 总时长（秒）
    val buffered: Double   // 缓冲进度（秒）
)

/**
 * 播放器配置
 */
@Serializable
data class PlayerConfig(
    val volume: Float = 1.0f,
    val playMode: PlayMode = PlayMode.Order,
    val playQuality: MusicQuality = MusicQuality.Hq,
    val audioOffload: Boolean = true,
    val handleAudioFocus: Boolean = true,
    val maxCacheSize: Long = 1024 // MB
)

/**
 * 歌词信息
 */
@Serializable
data class LyricInfo(
    val lyric: String,           // 原歌词 (LRC格式)
    val tlyric: String? = null,  // 翻译歌词
    val rlyric: String? = null,  // 罗马音歌词
    val lxlyric: String? = null  // 逐字歌词 (LX格式)
)

/**
 * 歌词行
 */
@Serializable
data class LyricLine(
    val startTime: Double,    // 开始时间（毫秒）
    val duration: Double,     // 持续时间（毫秒）
    val text: String,         // 歌词文本
    val translation: String? = null,  // 翻译
    val romaji: String? = null,       // 罗马音
    val words: List<WordTiming> = emptyList()  // 逐字时间信息
)

/**
 * 逐字时间信息
 */
@Serializable
data class WordTiming(
    val startChar: Int,       // 字在歌词中的起始位置
    val charCount: Int,       // 字的长度
    val startTime: Double,    // 开始时间（毫秒）
    val duration: Double,     // 持续时间（毫秒）
    val text: String          // 字内容
)

/**
 * 解析后的歌词
 */
@Serializable
data class ParsedLyric(
    val title: String? = null,
    val artist: String? = null,
    val album: String? = null,
    val lyricist: String? = null,
    val length: Double? = null,
    val offset: Double = 0.0,
    val lines: List<LyricLine> = emptyList(),
    val hasWordTiming: Boolean = false
)

/**
 * 搜索结果
 */
@Serializable
data class SearchResult(
    val list: List<MusicInfo>,
    val total: Int,
    val source: MusicSource
)

/**
 * 播放列表项
 */
@Serializable
data class PlaylistItem(
    val musicInfo: MusicInfo,
    val url: String? = null
)
