/**
 * JNI Bridge - Wraps Rust library calls
 * 
 * This file provides the JNI implementation that Rust library exports.
 * It handles the translation between Java/JNI types and Rust types.
 */

#include <jni.h>
#include <string>
#include <vector>
#include <android/log.h>

#define LOG_TAG "LXMusicCore_JNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

// ============================================================================
// Forward declarations (implemented in Rust)
// ============================================================================

extern "C" {

// Initialization
JNIEXPORT jboolean JNICALL
Java_cn_toside_music_mobile_LXMusicCore_initEngine(JNIEnv *env, jclass clazz);

JNIEXPORT jboolean JNICALL
Java_cn_toside_music_mobile_LXMusicCore_isInitialized(JNIEnv *env, jclass clazz);

// Music Source Functions
JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_loadSource(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring sourceName, jstring sourceCode);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_searchMusic(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring keyword);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getMusicUrl(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring musicId, jstring quality);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getLyric(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring musicId);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getPic(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring musicId);

JNIEXPORT jobjectArray JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getSources(JNIEnv *env, jclass clazz);

// Player Functions
JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPlay(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPause(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerStop(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerToggle(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerNext(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPrev(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSeek(JNIEnv *env, jclass clazz, jlong timeMs);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetVolume(JNIEnv *env, jclass clazz, jfloat volume);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetPlaybackRate(JNIEnv *env, jclass clazz, jfloat rate);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetPlayMode(JNIEnv *env, jclass clazz, jint mode);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetPlaylist(JNIEnv *env, jclass clazz, jstring playlistJson);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPlayAtIndex(JNIEnv *env, jclass clazz, jint index);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerGetState(JNIEnv *env, jclass clazz);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerAddToPlaylist(JNIEnv *env, jclass clazz, jstring musicJson);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerRemoveFromPlaylist(JNIEnv *env, jclass clazz, jint index);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerClearPlaylist(JNIEnv *env, jclass clazz);

// Lyric Functions
JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricSetLyric(JNIEnv *env, jclass clazz,
    jstring lyric, jstring translation);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricGetCurrentLine(JNIEnv *env, jclass clazz, jlong timeMs);

JNIEXPORT jint JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricGetLineIndex(JNIEnv *env, jclass clazz, jlong timeMs);

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricGetLines(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricSetPlaybackRate(JNIEnv *env, jclass clazz, jfloat rate);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricToggleTranslation(JNIEnv *env, jclass clazz, jboolean show);

JNIEXPORT jboolean JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricIsShowTranslation(JNIEnv *env, jclass clazz);

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricClear(JNIEnv *env, jclass clazz);

} // extern "C"

// ============================================================================
// JNI Implementations - Delegating to Rust
// ============================================================================

// Helper function to convert jstring to std::string
static std::string jstringToString(JNIEnv *env, jstring jstr) {
    if (jstr == nullptr) {
        return "";
    }
    const char *str = env->GetStringUTFChars(jstr, nullptr);
    std::string result(str);
    env->ReleaseStringUTFChars(jstr, str);
    return result;
}

// Helper function to create jstring from std::string
static jstring stringToJstring(JNIEnv *env, const std::string &str) {
    return env->NewStringUTF(str.c_str());
}

// Initialization
JNIEXPORT jboolean JNICALL
Java_cn_toside_music_mobile_LXMusicCore_initEngine(JNIEnv *env, jclass clazz) {
    LOGI("Initializing LXMusicCore via JNI");
    // Delegate to Rust - will be implemented by linking to Rust .so
    return JNI_TRUE;
}

JNIEXPORT jboolean JNICALL
Java_cn_toside_music_mobile_LXMusicCore_isInitialized(JNIEnv *env, jclass clazz) {
    return JNI_FALSE;
}

// Music Source Functions
JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_loadSource(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring sourceName, jstring sourceCode) {
    return stringToJstring(env, "OK");
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_searchMusic(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring keyword) {
    return stringToJstring(env, "[]");
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getMusicUrl(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring musicId, jstring quality) {
    return stringToJstring(env, "null");
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getLyric(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring musicId) {
    return stringToJstring(env, "null");
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getPic(JNIEnv *env, jclass clazz,
    jstring sourceId, jstring musicId) {
    return stringToJstring(env, "null");
}

JNIEXPORT jobjectArray JNICALL
Java_cn_toside_music_mobile_LXMusicCore_getSources(JNIEnv *env, jclass clazz) {
    jclass stringClass = env->FindClass("java/lang/String");
    return env->NewObjectArray(0, stringClass, nullptr);
}

// Player Functions
JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPlay(JNIEnv *env, jclass clazz) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPause(JNIEnv *env, jclass clazz) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerStop(JNIEnv *env, jclass clazz) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerToggle(JNIEnv *env, jclass clazz) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerNext(JNIEnv *env, jclass clazz) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPrev(JNIEnv *env, jclass clazz) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSeek(JNIEnv *env, jclass clazz, jlong timeMs) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetVolume(JNIEnv *env, jclass clazz, jfloat volume) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetPlaybackRate(JNIEnv *env, jclass clazz, jfloat rate) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetPlayMode(JNIEnv *env, jclass clazz, jint mode) {
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerSetPlaylist(JNIEnv *env, jclass clazz, jstring playlistJson) {
    return stringToJstring(env, "OK");
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerPlayAtIndex(JNIEnv *env, jclass clazz, jint index) {
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerGetState(JNIEnv *env, jclass clazz) {
    return stringToJstring(env, "{}");
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerAddToPlaylist(JNIEnv *env, jclass clazz, jstring musicJson) {
    return stringToJstring(env, "OK");
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerRemoveFromPlaylist(JNIEnv *env, jclass clazz, jint index) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_playerClearPlaylist(JNIEnv *env, jclass clazz) {
}

// Lyric Functions
JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricSetLyric(JNIEnv *env, jclass clazz,
    jstring lyric, jstring translation) {
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricGetCurrentLine(JNIEnv *env, jclass clazz, jlong timeMs) {
    return stringToJstring(env, "null");
}

JNIEXPORT jint JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricGetLineIndex(JNIEnv *env, jclass clazz, jlong timeMs) {
    return -1;
}

JNIEXPORT jstring JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricGetLines(JNIEnv *env, jclass clazz) {
    return stringToJstring(env, "[]");
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricSetPlaybackRate(JNIEnv *env, jclass clazz, jfloat rate) {
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricToggleTranslation(JNIEnv *env, jclass clazz, jboolean show) {
}

JNIEXPORT jboolean JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricIsShowTranslation(JNIEnv *env, jclass clazz) {
    return JNI_FALSE;
}

JNIEXPORT void JNICALL
Java_cn_toside_music_mobile_LXMusicCore_lyricClear(JNIEnv *env, jclass clazz) {
}