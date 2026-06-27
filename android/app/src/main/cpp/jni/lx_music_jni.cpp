#include <jni.h>
#include <android/log.h>

#define LOG_TAG "LXMusicJNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

extern "C" {
    JNIEXPORT jstring JNICALL
    Java_cn_toside_music_mobile_LXMusicCore_nativeInit(JNIEnv* env, jobject thiz) {
        LOGI("Native init called");
        return env->NewStringUTF("Rust core initialized");
    }

    JNIEXPORT jstring JNICALL
    Java_cn_toside_music_mobile_LXMusicCore_nativeTest(JNIEnv* env, jobject thiz, jstring input) {
        const char* inputStr = env->GetStringUTFChars(input, nullptr);
        LOGI("Native test called with: %s", inputStr);
        env->ReleaseStringUTFChars(input, inputStr);
        return env->NewStringUTF("Test successful");
    }
}