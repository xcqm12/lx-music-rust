use std::fmt;

#[derive(Debug)]
pub enum Error {
    // 播放错误
    Player(String),
    
    // 音乐源错误
    MusicSource(String),
    SourceNotFound(String),
    RequestFailed(String),
    ParseFailed(String),
    
    // 歌词错误
    Lyric(String),
    LyricNotFound,
    LyricParseFailed(String),
    
    // 网络错误
    Network(String),
    
    // IO 错误
    Io(std::io::Error),
    
    // 序列化错误
    Serialization(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Player(msg) => write!(f, "Player error: {}", msg),
            Error::MusicSource(msg) => write!(f, "Music source error: {}", msg),
            Error::SourceNotFound(source) => write!(f, "Source not found: {}", source),
            Error::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            Error::ParseFailed(msg) => write!(f, "Parse failed: {}", msg),
            Error::Lyric(msg) => write!(f, "Lyric error: {}", msg),
            Error::LyricNotFound => write!(f, "Lyric not found"),
            Error::LyricParseFailed(msg) => write!(f, "Lyric parse failed: {}", msg),
            Error::Network(msg) => write!(f, "Network error: {}", msg),
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Network(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
