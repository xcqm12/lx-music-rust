package com.lx.music.musicsource

import com.lx.music.core.model.LyricInfo
import com.lx.music.core.model.MusicInfo
import com.lx.music.core.model.MusicQuality
import com.lx.music.core.model.SearchResult
import kotlinx.serialization.json.Json
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * 音乐源存储库
 * 负责与 Rust 音乐源模块通信
 */
@Singleton
class MusicSourceRepository @Inject constructor() {
    
    private val json = Json { ignoreUnknownKeys = true }
    
    init {
        MusicSourceBridge.initialize()
    }
    
    /**
     * 搜索音乐
     */
    suspend fun search(keyword: String): List<MusicInfo> {
        return try {
            // 搜索所有源
            val allResults = mutableListOf<MusicInfo>()
            val sources = listOf("kw", "kg", "tx", "wy", "mg")
            
            for (source in sources) {
                try {
                    val resultJson = MusicSourceBridge.search(source, keyword, 1, 20)
                    val results = json.decodeFromString<List<MusicInfo>>(resultJson)
                    allResults.addAll(results)
                } catch (e: Exception) {
                    Timber.w(e, "Search failed for source: $source")
                }
            }
            
            allResults
        } catch (e: Exception) {
            Timber.e(e, "Search failed")
            emptyList()
        }
    }
    
    /**
     * 获取音乐 URL
     */
    suspend fun getMusicUrl(musicInfo: MusicInfo, quality: MusicQuality = MusicQuality.Hq): String? {
        return try {
            MusicSourceBridge.getMusicUrl(musicInfo, quality)
        } catch (e: Exception) {
            Timber.e(e, "Failed to get music URL")
            null
        }
    }
    
    /**
     * 获取歌词
     */
    suspend fun getLyric(musicInfo: MusicInfo): LyricInfo? {
        return try {
            val lyricJson = MusicSourceBridge.getLyric(musicInfo)
            json.decodeFromString<LyricInfo>(lyricJson)
        } catch (e: Exception) {
            Timber.e(e, "Failed to get lyric")
            null
        }
    }
    
    /**
     * 跨源查找音乐
     */
    suspend fun findMusicCrossSource(musicInfo: MusicInfo): List<MusicInfo> {
        return try {
            val resultJson = MusicSourceBridge.findMusicCrossSource(musicInfo)
            json.decodeFromString<List<MusicInfo>>(resultJson)
        } catch (e: Exception) {
            Timber.e(e, "Cross source search failed")
            emptyList()
        }
    }
    
    /**
     * 获取推荐歌曲
     */
    suspend fun getRecommendations(): List<MusicInfo> {
        // 返回一些默认推荐
        return emptyList()
    }
    
    /**
     * 获取热门搜索
     */
    suspend fun getHotSearches(): List<String> {
        // 返回一些热门搜索词
        return listOf(
            "周杰伦",
            "薛之谦",
            "邓紫棋",
            "林俊杰",
            "陈奕迅"
        )
    }
}
