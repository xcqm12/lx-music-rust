package cn.toside.music.mobile

/**
 * RustBridge - JNI Bridge to Rust Core Library
 * 
 * This class provides static native methods that are implemented in Rust.
 * All methods delegate to the liblx_music_core.so native library.
 * 
 * Architecture: Java -> JNI -> Rust .so
 */
object RustBridge {

    /**
     * 可恢复式的原生库加载：
     * - 库缺失（如个别 ABI 未打包）或加载失败时不再抛出 Error 导致调用线程崩溃；
     * - Kotlin object 初始化失败会被 JVM 永久毒化（后续访问一直抛
     *   ExceptionInInitializerError），因此这里在 init 内部吞掉异常，
     *   由调用方通过 [isAvailable] 判断并降级。
     */
    @Volatile
    var isAvailable: Boolean = false
        private set

    private val loadLock = Any()

    @Volatile
    private var loadAttempted = false

    private fun ensureLoaded(): Boolean {
        if (loadAttempted) return isAvailable
        synchronized(loadLock) {
            if (loadAttempted) return isAvailable
            loadAttempted = true
            try {
                System.loadLibrary("lx_music_core")
                isAvailable = true
            } catch (t: Throwable) {
                isAvailable = false
                android.util.Log.e("RustBridge", "Failed to load liblx_music_core.so, rust features disabled", t)
            }
        }
        return isAvailable
    }

    init {
        ensureLoaded()
    }

    // ========================================================================
    // Initialization
    // ========================================================================

    /**
     * Initialize the Rust engine
     * @return true if initialization was successful
     */
    external fun initEngine(): Boolean

    /**
     * Check if the Rust engine is initialized
     * @return true if initialized
     */
    external fun isInitialized(): Boolean

    // ========================================================================
    // Music Source Functions
    // ========================================================================

    /**
     * Load a music source script
     * @param sourceId Unique identifier for the source
     * @param sourceName Display name for the source
     * @param sourceCode JavaScript source code
     * @return "OK" on success, error message otherwise
     */
    external fun loadSource(sourceId: String, sourceName: String, sourceCode: String): String

    /**
     * Search for music on a source
     * @param sourceId Source identifier
     * @param keyword Search keyword
     * @return JSON array of music results
     */
    external fun searchMusic(sourceId: String, keyword: String): String

    /**
     * Get music URL from source
     * @param sourceId Source identifier
     * @param musicId Music identifier
     * @param quality Quality level (e.g., "low", "medium", "high")
     * @return JSON object with url field
     */
    external fun getMusicUrl(sourceId: String, musicId: String, quality: String): String

    /**
     * Get lyric from source
     * @param sourceId Source identifier
     * @param musicId Music identifier
     * @return JSON object with lyric fields
     */
    external fun getLyric(sourceId: String, musicId: String): String

    /**
     * Get album artwork URL from source
     * @param sourceId Source identifier
     * @param musicId Music identifier
     * @return JSON object with picUrl field
     */
    external fun getPic(sourceId: String, musicId: String): String

    /**
     * Get all loaded music sources
     * @return Array of [sourceId, sourceName] pairs
     */
    external fun getSources(): Array<String>

    // ========================================================================
    // Player Functions
    // ========================================================================

    /**
     * Start playback
     */
    external fun playerPlay()

    /**
     * Pause playback
     */
    external fun playerPause()

    /**
     * Stop playback
     */
    external fun playerStop()

    /**
     * Toggle play/pause
     */
    external fun playerToggle()

    /**
     * Play next track
     */
    external fun playerNext()

    /**
     * Play previous track
     */
    external fun playerPrev()

    /**
     * Seek to position
     * @param timeMs Position in milliseconds
     */
    external fun playerSeek(timeMs: Long)

    /**
     * Set volume
     * @param volume Volume level (0.0 - 1.0)
     */
    external fun playerSetVolume(volume: Float)

    /**
     * Set playback rate
     * @param rate Playback rate (0.5 - 2.0)
     */
    external fun playerSetPlaybackRate(rate: Float)

    /**
     * Set play mode
     * @param mode Play mode (0: list loop, 1: random, 2: list, 3: single loop)
     */
    external fun playerSetPlayMode(mode: Int)

    /**
     * Set playlist
     * @param playlistJson JSON array of music items
     * @return "OK" on success, error message otherwise
     */
    external fun playerSetPlaylist(playlistJson: String): String

    /**
     * Play specific track by index
     * @param index Track index in playlist
     */
    external fun playerPlayAtIndex(index: Int)

    /**
     * Get current player state
     * @return JSON object with player state
     */
    external fun playerGetState(): String

    /**
     * Add music to playlist
     * @param musicJson JSON object with music info
     * @return "OK" on success, error message otherwise
     */
    external fun playerAddToPlaylist(musicJson: String): String

    /**
     * Remove music from playlist
     * @param index Index to remove
     */
    external fun playerRemoveFromPlaylist(index: Int)

    /**
     * Clear playlist
     */
    external fun playerClearPlaylist()

    // ========================================================================
    // Lyric Functions
    // ========================================================================

    /**
     * Set lyric content
     * @param lyric LRC format lyric content
     * @param translation Translation content (optional)
     */
    external fun lyricSetLyric(lyric: String, translation: String)

    /**
     * Get current lyric line at time
     * @param timeMs Current time in milliseconds
     * @return JSON object with lyric line or null
     */
    external fun lyricGetCurrentLine(timeMs: Long): String

    /**
     * Get current lyric line index at time
     * @param timeMs Current time in milliseconds
     * @return Line index or -1
     */
    external fun lyricGetLineIndex(timeMs: Long): Int

    /**
     * Get all lyric lines
     * @return JSON array of lyric lines
     */
    external fun lyricGetLines(): String

    /**
     * Set lyric playback rate
     * @param rate Playback rate (0.5 - 2.0)
     */
    external fun lyricSetPlaybackRate(rate: Float)

    /**
     * Toggle translation display
     * @param show Whether to show translation
     */
    external fun lyricToggleTranslation(show: Boolean)

    /**
     * Check if translation is shown
     * @return true if translation is shown
     */
    external fun lyricIsShowTranslation(): Boolean

    /**
     * Clear lyric content
     */
    external fun lyricClear()
}