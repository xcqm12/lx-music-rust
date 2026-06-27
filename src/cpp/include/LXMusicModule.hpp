/**
 * LX Music TurboModule - C++ Bridge
 * 
 * React Native TurboModule that provides a type-safe bridge between
 * JavaScript and the Rust core library via JNI.
 * 
 * Architecture: JS -> C++ TurboModule -> JNI -> Rust .so
 */

#pragma once

#include <string>
#include <memory>
#include <jni.h>

namespace lxmusic {

/**
 * Native module configuration for React Native codegen
 */
class LXMusicModuleConfig {
public:
    static constexpr const char* Name = "LXMusicCore";
    static constexpr auto moduleId = "LXMusicCoreModule";
    
    // Method names for codegen
    struct Methods {
        static constexpr const char* InitEngine = "initEngine";
        static constexpr const char* IsInitialized = "isInitialized";
        
        // Music source methods
        static constexpr const char* LoadSource = "loadSource";
        static constexpr const char* SearchMusic = "searchMusic";
        static constexpr const char* GetMusicUrl = "getMusicUrl";
        static constexpr const char* GetLyric = "getLyric";
        static constexpr const char* GetPic = "getPic";
        static constexpr const char* GetSources = "getSources";
        
        // Player methods
        static constexpr const char* PlayerPlay = "playerPlay";
        static constexpr const char* PlayerPause = "playerPause";
        static constexpr const char* PlayerStop = "playerStop";
        static constexpr const char* PlayerToggle = "playerToggle";
        static constexpr const char* PlayerNext = "playerNext";
        static constexpr const char* PlayerPrev = "playerPrev";
        static constexpr const char* PlayerSeek = "playerSeek";
        static constexpr const char* PlayerSetVolume = "playerSetVolume";
        static constexpr const char* PlayerSetPlaybackRate = "playerSetPlaybackRate";
        static constexpr const char* PlayerSetPlayMode = "playerSetPlayMode";
        static constexpr const char* PlayerSetPlaylist = "playerSetPlaylist";
        static constexpr const char* PlayerPlayAtIndex = "playerPlayAtIndex";
        static constexpr const char* PlayerGetState = "playerGetState";
        static constexpr const char* PlayerAddToPlaylist = "playerAddToPlaylist";
        static constexpr const char* PlayerRemoveFromPlaylist = "playerRemoveFromPlaylist";
        static constexpr const char* PlayerClearPlaylist = "playerClearPlaylist";
        
        // Lyric methods
        static constexpr const char* LyricSetLyric = "lyricSetLyric";
        static constexpr const char* LyricGetCurrentLine = "lyricGetCurrentLine";
        static constexpr const char* LyricGetLineIndex = "lyricGetLineIndex";
        static constexpr const char* LyricGetLines = "lyricGetLines";
        static constexpr const char* LyricSetPlaybackRate = "lyricSetPlaybackRate";
        static constexpr const char* LyricToggleTranslation = "lyricToggleTranslation";
        static constexpr const char* LyricIsShowTranslation = "lyricIsShowTranslation";
        static constexpr const char* LyricClear = "lyricClear";
    };
};

/**
 * JNI Bridge wrapper for Rust functions
 */
class JniBridge {
public:
    static bool initialize(JavaVM* vm);
    static bool isInitialized();
    
    // Music source functions
    static std::string loadSource(const std::string& sourceId, 
                                  const std::string& sourceName, 
                                  const std::string& sourceCode);
    static std::string searchMusic(const std::string& sourceId, 
                                   const std::string& keyword);
    static std::string getMusicUrl(const std::string& sourceId, 
                                  const std::string& musicId, 
                                  const std::string& quality);
    static std::string getLyric(const std::string& sourceId, 
                               const std::string& musicId);
    static std::string getPic(const std::string& sourceId, 
                             const std::string& musicId);
    static std::vector<std::pair<std::string, std::string>> getSources();
    
    // Player functions
    static void playerPlay();
    static void playerPause();
    static void playerStop();
    static void playerToggle();
    static void playerNext();
    static void playerPrev();
    static void playerSeek(int64_t timeMs);
    static void playerSetVolume(float volume);
    static void playerSetPlaybackRate(float rate);
    static void playerSetPlayMode(int mode);
    static std::string playerSetPlaylist(const std::string& playlistJson);
    static void playerPlayAtIndex(int index);
    static std::string playerGetState();
    static std::string playerAddToPlaylist(const std::string& musicJson);
    static void playerRemoveFromPlaylist(int index);
    static void playerClearPlaylist();
    
    // Lyric functions
    static void lyricSetLyric(const std::string& lyric, 
                            const std::string& translation);
    static std::string lyricGetCurrentLine(int64_t timeMs);
    static int lyricGetLineIndex(int64_t timeMs);
    static std::string lyricGetLines();
    static void lyricSetPlaybackRate(float rate);
    static void lyricToggleTranslation(bool show);
    static bool lyricIsShowTranslation();
    static void lyricClear();

private:
    static JavaVM* g_jvm;
    static bool initJni();
};

} // namespace lxmusic