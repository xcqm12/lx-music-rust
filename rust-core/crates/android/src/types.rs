use jni::objects::JString;
use jni::signature::JavaType;
use jni::JNIEnv;

/// 将 Rust 字符串转换为 JNI 字符串
pub fn to_jstring<'local>(env: &mut JNIEnv<'local>, s: &str) -> JString<'local> {
    env.new_string(s)
        .expect("Failed to create Java string")
        .into()
}

/// 从 JNI 字符串获取 Rust 字符串
pub fn from_jstring<'local>(env: &mut JNIEnv<'local>, s: &JString<'local>) -> Option<String> {
    if s.is_null() {
        return None;
    }
    env.get_string(s)
        .ok()
        .map(|js| js.to_string_lossy().to_string())
}

/// JNI 错误类型
#[derive(Debug)]
pub enum JniError {
    NullPointer,
    InvalidUtf8,
    TypeMismatch,
    ExceptionThrown,
}

impl std::fmt::Display for JniError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JniError::NullPointer => write!(f, "Null pointer"),
            JniError::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            JniError::TypeMismatch => write!(f, "Type mismatch"),
            JniError::ExceptionThrown => write!(f, "Java exception thrown"),
        }
    }
}

impl std::error::Error for JniError {}

/// 结果类型
pub type JniResult<T> = Result<T, JniError>;

/// 序列化结果到 JSON 字符串
pub fn to_json<'local, T: serde::Serialize>(env: &mut JNIEnv<'local>, value: &T) -> JString<'local> {
    match serde_json::to_string(value) {
        Ok(json) => to_jstring(env, &json),
        Err(e) => {
            log::error!("Failed to serialize to JSON: {}", e);
            to_jstring(env, "{}")
        }
    }
}

/// 从 JSON 字符串反序列化
pub fn from_json<'local, T: serde::de::DeserializeOwned>(
    env: &mut JNIEnv<'local>,
    jstr: &JString<'local>,
) -> JniResult<T> {
    let json_str = from_jstring(env, jstr).ok_or(JniError::NullPointer)?;
    serde_json::from_str(&json_str).map_err(|_| JniError::InvalidUtf8)
}
