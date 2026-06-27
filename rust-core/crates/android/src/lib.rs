pub mod player_ffi;
pub mod music_source_ffi;
pub mod lyric_ffi;
pub mod types;

use jni::JNIEnv;
use jni::objects::JString;
use jni::signature::JavaType;
use jni::signature::Primitive;
use log::Level;

/// 初始化日志系统
#[no_mangle]
pub extern "C" fn Java_com_lx_music_core_RustCoreBridge_initLogging(_env: JNIEnv<'_>, _class: jni::objects::JClass<'_>) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug),
    );
}

/// 版本信息
#[no_mangle]
pub extern "C" fn Java_com_lx_music_core_RustCoreBridge_getVersion<'local>(
    mut env: JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
) -> JString<'local> {
    let version = env.new_string("0.1.0")
        .expect("Failed to create version string");
    version.into()
}
