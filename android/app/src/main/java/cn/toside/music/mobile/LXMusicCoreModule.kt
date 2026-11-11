package cn.toside.music.mobile

import com.facebook.react.bridge.*
import com.facebook.react.module.annotations.ReactModule

/**
 * LXMusicCore React Native Module
 * 
 * Provides a bridge between React Native JavaScript and the Rust core library.
 * This module delegates all calls to the Rust library via JNI.
 * 
 * Architecture: JS -> ReactModule -> JNI -> Rust .so
 */
@ReactModule(name = "LXMusicCore")
class LXMusicCoreModule(reactContext: ReactApplicationContext) : ReactContextBaseJavaModule(reactContext) {

    companion object {
        const val NAME = "LXMusicCore"
    }

    override fun getName(): String = NAME

    // ========================================================================
    // Initialization
    // ========================================================================

    @ReactMethod
    fun initEngine(promise: Promise) {
        try {
            val result = RustBridge.initEngine()
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("INIT_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun isInitialized(promise: Promise) {
        try {
            val result = RustBridge.isInitialized()
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("INIT_CHECK_ERROR", e.message, e)
        }
    }

    // ========================================================================
    // Music Source Functions
    // ========================================================================

    @ReactMethod
    fun loadSource(sourceId: String, sourceName: String, sourceCode: String, promise: Promise) {
        try {
            val result = RustBridge.loadSource(sourceId, sourceName, sourceCode)
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("LOAD_SOURCE_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun searchMusic(sourceId: String, keyword: String, promise: Promise) {
        try {
            val result = RustBridge.searchMusic(sourceId, keyword)
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("SEARCH_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun getMusicUrl(sourceId: String, musicId: String, quality: String, promise: Promise) {
        try {
            val result = RustBridge.getMusicUrl(sourceId, musicId, quality)
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("GET_URL_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun getLyric(sourceId: String, musicId: String, promise: Promise) {
        try {
            val result = RustBridge.getLyric(sourceId, musicId)
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("GET_LYRIC_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun getPic(sourceId: String, musicId: String, promise: Promise) {
        try {
            val result = RustBridge.getPic(sourceId, musicId)
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("GET_PIC_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun getSources(promise: Promise) {
        try {
            val result = RustBridge.getSources()
            promise.resolve(Arguments.createArray().apply {
                result.forEach { pushString(it) }
            })
        } catch (e: Throwable) {
            promise.reject("GET_SOURCES_ERROR", e.message, e)
        }
    }

    // ========================================================================
    // Player Functions
    // ========================================================================

    @ReactMethod
    fun playerPlay() {
        try {
            RustBridge.playerPlay()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerPause() {
        try {
            RustBridge.playerPause()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerStop() {
        try {
            RustBridge.playerStop()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerToggle() {
        try {
            RustBridge.playerToggle()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerNext() {
        try {
            RustBridge.playerNext()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerPrev() {
        try {
            RustBridge.playerPrev()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerSeek(timeMs: Double) {
        try {
            RustBridge.playerSeek(timeMs.toLong())
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerSetVolume(volume: Float) {
        try {
            RustBridge.playerSetVolume(volume)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerSetPlaybackRate(rate: Float) {
        try {
            RustBridge.playerSetPlaybackRate(rate)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerSetPlayMode(mode: Int) {
        try {
            RustBridge.playerSetPlayMode(mode)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerSetPlaylist(playlistJson: String, promise: Promise) {
        try {
            val result = RustBridge.playerSetPlaylist(playlistJson)
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("SET_PLAYLIST_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun playerPlayAtIndex(index: Int) {
        try {
            RustBridge.playerPlayAtIndex(index)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerGetState(promise: Promise) {
        try {
            val result = RustBridge.playerGetState()
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("GET_STATE_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun playerAddToPlaylist(musicJson: String, promise: Promise) {
        try {
            val result = RustBridge.playerAddToPlaylist(musicJson)
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("ADD_TO_PLAYLIST_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun playerRemoveFromPlaylist(index: Int) {
        try {
            RustBridge.playerRemoveFromPlaylist(index)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun playerClearPlaylist() {
        try {
            RustBridge.playerClearPlaylist()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    // ========================================================================
    // Lyric Functions
    // ========================================================================

    @ReactMethod
    fun lyricSetLyric(lyric: String, translation: String) {
        try {
            RustBridge.lyricSetLyric(lyric, translation)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun lyricGetCurrentLine(timeMs: Double, promise: Promise) {
        try {
            val result = RustBridge.lyricGetCurrentLine(timeMs.toLong())
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("GET_CURRENT_LINE_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun lyricGetLineIndex(timeMs: Double, promise: Promise) {
        try {
            val result = RustBridge.lyricGetLineIndex(timeMs.toLong())
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("GET_LINE_INDEX_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun lyricGetLines(promise: Promise) {
        try {
            val result = RustBridge.lyricGetLines()
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("GET_LINES_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun lyricSetPlaybackRate(rate: Float) {
        try {
            RustBridge.lyricSetPlaybackRate(rate)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun lyricToggleTranslation(show: Boolean) {
        try {
            RustBridge.lyricToggleTranslation(show)
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }

    @ReactMethod
    fun lyricIsShowTranslation(promise: Promise) {
        try {
            val result = RustBridge.lyricIsShowTranslation()
            promise.resolve(result)
        } catch (e: Throwable) {
            promise.reject("IS_SHOW_TRANSLATION_ERROR", e.message, e)
        }
    }

    @ReactMethod
    fun lyricClear() {
        try {
            RustBridge.lyricClear()
        } catch (e: Throwable) {
            // 捕获 Throwable：库缺失时抛出的 UnsatisfiedLinkError 是 Error 不是 Exception，必须拦住避免 RN 线程死亡
            android.util.Log.w("LXMusicCore", "rust call failed (available=${RustBridge.isAvailable})", e)
        }
    }
}