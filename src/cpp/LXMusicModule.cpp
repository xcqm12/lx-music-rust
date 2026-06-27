/**
 * LX Music TurboModule - C++ Implementation
 * 
 * Implements the JNI bridge to Rust core library and provides
 * type-safe interface for React Native.
 */

#include "LXMusicModule.hpp"
#include <android/log.h>
#include <vector>

#define LOG_TAG "LXMusicCore"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

namespace lxmusic {

JavaVM* JniBridge::g_jvm = nullptr;

/**
 * Initialize JNI environment
 */
bool JniBridge::initialize(JavaVM* vm) {
    g_jvm = vm;
    LOGI("JNI Bridge initialized with VM: %p", vm);
    return initJni();
}

/**
 * Initialize JNI and call Rust engine initialization
 */
bool JniBridge::initJni() {
    if (!g_jvm) {
        LOGE("JavaVM is null");
        return false;
    }

    JNIEnv* env = nullptr;
    jint result = g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    
    if (result == JNI_EDETACHED) {
        result = g_jvm->AttachCurrentThread(&env, nullptr);
        if (result != JNI_OK) {
            LOGE("Failed to attach current thread");
            return false;
        }
    }

    // Find the native library class
    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (env->ExceptionCheck()) {
        env->ExceptionDescribe();
        env->ExceptionClear();
        LOGE("Exception finding LXMusicCore class");
        return false;
    }

    LOGI("JNI initialization successful");
    return true;
}

bool JniBridge::isInitialized() {
    if (!g_jvm) return false;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return false;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return false;

    jmethodID method = env->GetStaticMethodID(nativeClass, "isInitialized", "()Z");
    if (!method) return false;

    jboolean result = env->CallStaticBooleanMethod(nativeClass, method);
    return result == JNI_TRUE;
}

// ============================================================================
// Music Source Functions
// ============================================================================

std::string JniBridge::loadSource(const std::string& sourceId, 
                                  const std::string& sourceName, 
                                  const std::string& sourceCode) {
    if (!g_jvm) return "Error: JVM not initialized";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "Error: Cannot get JNIEnv";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "Error: Class not found";

    jmethodID method = env->GetStaticMethodID(nativeClass, "loadSource",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;");
    if (!method) return "Error: Method not found";

    jstring jSourceId = env->NewStringUTF(sourceId.c_str());
    jstring jSourceName = env->NewStringUTF(sourceName.c_str());
    jstring jSourceCode = env->NewStringUTF(sourceCode.c_str());

    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method,
        jSourceId, jSourceName, jSourceCode);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(jSourceId, sourceId.c_str());
    env->ReleaseStringUTFChars(jSourceName, sourceName.c_str());
    env->ReleaseStringUTFChars(jSourceCode, sourceCode.c_str());
    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

std::string JniBridge::searchMusic(const std::string& sourceId, 
                                   const std::string& keyword) {
    if (!g_jvm) return "[]";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "[]";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "[]";

    jmethodID method = env->GetStaticMethodID(nativeClass, "searchMusic",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;");
    if (!method) return "[]";

    jstring jSourceId = env->NewStringUTF(sourceId.c_str());
    jstring jKeyword = env->NewStringUTF(keyword.c_str());

    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method,
        jSourceId, jKeyword);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

std::string JniBridge::getMusicUrl(const std::string& sourceId, 
                                   const std::string& musicId, 
                                   const std::string& quality) {
    if (!g_jvm) return "null";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "null";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "null";

    jmethodID method = env->GetStaticMethodID(nativeClass, "getMusicUrl",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;");
    if (!method) return "null";

    jstring jSourceId = env->NewStringUTF(sourceId.c_str());
    jstring jMusicId = env->NewStringUTF(musicId.c_str());
    jstring jQuality = env->NewStringUTF(quality.c_str());

    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method,
        jSourceId, jMusicId, jQuality);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

std::string JniBridge::getLyric(const std::string& sourceId, 
                                const std::string& musicId) {
    if (!g_jvm) return "null";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "null";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "null";

    jmethodID method = env->GetStaticMethodID(nativeClass, "getLyric",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;");
    if (!method) return "null";

    jstring jSourceId = env->NewStringUTF(sourceId.c_str());
    jstring jMusicId = env->NewStringUTF(musicId.c_str());

    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method,
        jSourceId, jMusicId);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

std::vector<std::pair<std::string, std::string>> JniBridge::getSources() {
    std::vector<std::pair<std::string, std::string>> sources;

    if (!g_jvm) return sources;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return sources;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return sources;

    jmethodID method = env->GetStaticMethodID(nativeClass, "getSources",
        "()[Ljava/lang/String;");
    if (!method) return sources;

    jobjectArray result = (jobjectArray)env->CallStaticObjectMethod(nativeClass, method);
    if (!result) return sources;

    jsize length = env->GetArrayLength(result);
    for (jsize i = 0; i + 1 < length; i += 2) {
        jstring idStr = (jstring)env.GetObjectArrayElement(result, i);
        jstring nameStr = (jstring)env.GetObjectArrayElement(result, i + 1);
        
        const char* id = env->GetStringUTFChars(idStr, nullptr);
        const char* name = env->GetStringUTFChars(nameStr, nullptr);
        
        sources.push_back({id, name});
        
        env->ReleaseStringUTFChars(idStr, id);
        env->ReleaseStringUTFChars(nameStr, name);
    }

    return sources;
}

// ============================================================================
// Player Functions
// ============================================================================

void JniBridge::playerPlay() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerPlay", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

void JniBridge::playerPause() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerPause", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

void JniBridge::playerStop() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerStop", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

void JniBridge::playerToggle() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerToggle", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

void JniBridge::playerNext() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerNext", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

void JniBridge::playerPrev() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerPrev", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

void JniBridge::playerSeek(int64_t timeMs) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerSeek", "(J)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jlong)timeMs);
}

void JniBridge::playerSetVolume(float volume) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerSetVolume", "(F)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jfloat)volume);
}

void JniBridge::playerSetPlaybackRate(float rate) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerSetPlaybackRate", "(F)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jfloat)rate);
}

void JniBridge::playerSetPlayMode(int mode) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerSetPlayMode", "(I)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jint)mode);
}

std::string JniBridge::playerSetPlaylist(const std::string& playlistJson) {
    if (!g_jvm) return "Error: JVM not initialized";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "Error: Cannot get JNIEnv";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "Error: Class not found";

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerSetPlaylist",
        "(Ljava/lang/String;)Ljava/lang/String;");
    if (!method) return "Error: Method not found";

    jstring jPlaylist = env->NewStringUTF(playlistJson.c_str());
    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method, jPlaylist);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

void JniBridge::playerPlayAtIndex(int index) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerPlayAtIndex", "(I)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jint)index);
}

std::string JniBridge::playerGetState() {
    if (!g_jvm) return "{}";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "{}";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "{}";

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerGetState",
        "()Ljava/lang/String;");
    if (!method) return "{}";

    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

std::string JniBridge::playerAddToPlaylist(const std::string& musicJson) {
    if (!g_jvm) return "Error: JVM not initialized";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "Error: Cannot get JNIEnv";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "Error: Class not found";

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerAddToPlaylist",
        "(Ljava/lang/String;)Ljava/lang/String;");
    if (!method) return "Error: Method not found";

    jstring jMusic = env->NewStringUTF(musicJson.c_str());
    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method, jMusic);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

void JniBridge::playerRemoveFromPlaylist(int index) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerRemoveFromPlaylist", "(I)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jint)index);
}

void JniBridge::playerClearPlaylist() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "playerClearPlaylist", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

// ============================================================================
// Lyric Functions
// ============================================================================

void JniBridge::lyricSetLyric(const std::string& lyric, 
                               const std::string& translation) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricSetLyric",
        "(Ljava/lang/String;Ljava/lang/String;)V");
    if (!method) return;

    jstring jLyric = env->NewStringUTF(lyric.c_str());
    jstring jTranslation = env->NewStringUTF(translation.c_str());

    env->CallStaticVoidMethod(nativeClass, method, jLyric, jTranslation);
}

std::string JniBridge::lyricGetCurrentLine(int64_t timeMs) {
    if (!g_jvm) return "null";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "null";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "null";

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricGetCurrentLine",
        "(J)Ljava/lang/String;");
    if (!method) return "null";

    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method, (jlong)timeMs);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

int JniBridge::lyricGetLineIndex(int64_t timeMs) {
    if (!g_jvm) return -1;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return -1;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return -1;

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricGetLineIndex", "(J)I");
    if (!method) return -1;

    return (int)env->CallStaticIntMethod(nativeClass, method, (jlong)timeMs);
}

std::string JniBridge::lyricGetLines() {
    if (!g_jvm) return "[]";

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return "[]";

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return "[]";

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricGetLines",
        "()Ljava/lang/String;");
    if (!method) return "[]";

    jstring result = (jstring)env->CallStaticObjectMethod(nativeClass, method);

    std::string resultStr = env->GetStringUTFChars(result, nullptr);
    std::string ret = resultStr;

    env->ReleaseStringUTFChars(result, resultStr.c_str());

    return ret;
}

void JniBridge::lyricSetPlaybackRate(float rate) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricSetPlaybackRate", "(F)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jfloat)rate);
}

void JniBridge::lyricToggleTranslation(bool show) {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricToggleTranslation", "(Z)V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method, (jboolean)show);
}

bool JniBridge::lyricIsShowTranslation() {
    if (!g_jvm) return false;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return false;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return false;

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricIsShowTranslation", "()Z");
    if (!method) return false;

    return env->CallStaticBooleanMethod(nativeClass, method) == JNI_TRUE;
}

void JniBridge::lyricClear() {
    if (!g_jvm) return;

    JNIEnv* env = nullptr;
    g_jvm->GetEnv((void**)&env, JNI_VERSION_1_6);
    if (!env) return;

    jclass nativeClass = env->FindClass("cn/toside/music/mobile/LXMusicCore");
    if (!nativeClass) return;

    jmethodID method = env->GetStaticMethodID(nativeClass, "lyricClear", "()V");
    if (!method) return;

    env->CallStaticVoidMethod(nativeClass, method);
}

} // namespace lxmusic