package com.lx.music.player

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.media.MediaPlayer
import android.os.Build
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * 播放器存储库
 * 负责管理播放器状态和与 Rust 核心通信
 */
@Singleton
class PlayerRepository @Inject constructor(
    @ApplicationContext private val context: Context
) {
    private val _playerState = MutableStateFlow(PlayerState())
    val playerState: StateFlow<PlayerState> = _playerState.asStateFlow()
    
    private val _playlist = MutableStateFlow(PlaylistState())
    val playlist: StateFlow<PlaylistState> = _playlist.asStateFlow()
    
    private var mediaPlayer: MediaPlayer? = null
    private var audioManager: AudioManager = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private var audioFocusRequest: AudioFocusRequest? = null
    
    init {
        // 初始化 Rust 播放器
        initializePlayer()
    }
    
    private fun initializePlayer() {
        try {
            PlayerBridge.initialize(
                PlayerConfig(
                    volume = 1.0f,
                    playMode = PlayMode.Order,
                    playQuality = MusicQuality.Hq,
                    audioOffload = true,
                    handleAudioFocus = true,
                    maxCacheSize = 1024
                )
            )
        } catch (e: Exception) {
            Timber.e(e, "Failed to initialize player")
        }
    }
    
    suspend fun play(musicInfo: MusicInfo) {
        PlayerBridge.playTrack(musicInfo)
        
        // 获取音乐 URL
        val url = MusicSourceBridge.getMusicUrl(musicInfo, MusicQuality.Hq)
        
        // 使用 Android MediaPlayer 播放
        playWithMediaPlayer(url)
        
        _playerState.value = _playerState.value.copy(
            currentMusic = musicInfo,
            isPlaying = true
        )
    }
    
    private fun playWithMediaPlayer(url: String) {
        releaseMediaPlayer()
        
        mediaPlayer = MediaPlayer().apply {
            setAudioAttributes(
                AudioAttributes.Builder()
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .build()
            )
            
            setDataSource(url)
            prepareAsync()
            
            setOnPreparedListener { mp ->
                mp.start()
            }
            
            setOnCompletionListener {
                _playerState.value = _playerState.value.copy(isPlaying = false)
            }
            
            setOnErrorListener { _, what, extra ->
                Timber.e("MediaPlayer error: what=$what, extra=$extra")
                true
            }
        }
    }
    
    suspend fun togglePlayPause() {
        val currentState = _playerState.value
        if (currentState.isPlaying) {
            pause()
        } else {
            play()
        }
    }
    
    private suspend fun play() {
        PlayerBridge.play()
        mediaPlayer?.start()
        _playerState.value = _playerState.value.copy(isPlaying = true)
    }
    
    private fun pause() {
        PlayerBridge.pause()
        mediaPlayer?.pause()
        _playerState.value = _playerState.value.copy(isPlaying = false)
    }
    
    suspend fun playNext() {
        PlayerBridge.next()
        // 等待 Rust 返回下一首信息
    }
    
    suspend fun playPrevious() {
        PlayerBridge.previous()
    }
    
    fun seekTo(position: Float) {
        val positionMs = (position * 1000).toLong()
        PlayerBridge.seek(positionMs)
        mediaPlayer?.seekTo(positionMs.toInt())
    }
    
    fun setPlayMode(mode: PlayMode) {
        PlayerBridge.setPlayMode(mode)
        _playerState.value = _playerState.value.copy(playMode = mode)
    }
    
    suspend fun addToPlaylist(musicInfo: MusicInfo) {
        PlayerBridge.addToPlaylist(musicInfo)
        _playlist.value = _playlist.value.copy(
            items = _playlist.value.items + PlaylistItem(musicInfo)
        )
    }
    
    fun playAt(index: Int) {
        _playlist.value = _playlist.value.copy(currentIndex = index)
        _playlist.value.items.getOrNull(index)?.let { item ->
            // 播放指定索引的歌曲
        }
    }
    
    fun removeFromPlaylist(index: Int) {
        val newList = _playlist.value.items.toMutableList()
        newList.removeAt(index)
        _playlist.value = _playlist.value.copy(items = newList)
    }
    
    fun movePlaylistItem(fromIndex: Int, toIndex: Int) {
        val newList = _playlist.value.items.toMutableList()
        val item = newList.removeAt(fromIndex)
        newList.add(toIndex, item)
        _playlist.value = _playlist.value.copy(items = newList)
    }
    
    fun clearPlaylist() {
        _playlist.value = PlaylistState()
    }
    
    fun toggleFavorite(musicId: String, isFavorite: Boolean) {
        // 保存收藏状态
    }
    
    fun release() {
        releaseMediaPlayer()
        PlayerBridge.stop()
    }
    
    private fun releaseMediaPlayer() {
        mediaPlayer?.release()
        mediaPlayer = null
    }
}

// 数据类
data class PlayerState(
    val currentMusic: MusicInfo? = null,
    val isPlaying: Boolean = false,
    val position: Float = 0f,
    val duration: Float = 0f,
    val playMode: PlayMode = PlayMode.Order,
    val progress: Float = 0f
)

data class PlaylistState(
    val items: List<PlaylistItem> = emptyList(),
    val currentIndex: Int = -1
)
