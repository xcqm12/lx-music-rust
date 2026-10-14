pub mod sources;
pub mod crypto;
pub mod tx_lyric;
pub mod kw_lyric;
pub mod utils;

use async_trait::async_trait;
use common::{MusicInfo, MusicQuality, MusicSource, Result};
use reqwest::Client;
use std::sync::Arc;

/// 音乐源接口 trait
#[async_trait]
pub trait MusicSourceProvider: Send + Sync {
    /// 获取源名称
    fn name(&self) -> &str;
    
    /// 获取源 ID
    fn source_id(&self) -> MusicSource;
    
    /// 搜索音乐
    async fn search(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MusicInfo>>;
    
    /// 获取音乐 URL
    async fn get_music_url(
        &self,
        music_info: &MusicInfo,
        quality: MusicQuality,
    ) -> Result<String>;
    
    /// 获取歌词
    async fn get_lyric(
        &self,
        music_info: &MusicInfo,
    ) -> Result<common::LyricInfo>;
    
    /// 获取封面图片
    async fn get_pic_url(
        &self,
        music_info: &MusicInfo,
    ) -> Result<String>;
    
    /// 检查源是否可用
    async fn check_available(&self) -> Result<bool>;
}

/// 音乐源管理器
pub struct MusicSourceManager {
    http_client: Arc<Client>,
    sources: dashmap::DashMap<MusicSource, Arc<dyn MusicSourceProvider>>,
}

impl MusicSourceManager {
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.0")
            .build()
            .expect("Failed to create HTTP client");
        
        let manager = Self {
            http_client: Arc::new(http_client),
            sources: dashmap::DashMap::new(),
        };
        
        // 注册默认源
        manager.register_default_sources();
        
        manager
    }
    
    /// 注册默认音乐源
    fn register_default_sources(&self) {
        self.register(Arc::new(kw::KuwoSource::new(self.http_client.clone())));
        self.register(Arc::new(kg::KugouSource::new(self.http_client.clone())));
        self.register(Arc::new(tx::QQMusicSource::new(self.http_client.clone())));
        self.register(Arc::new(wy::NeteaseSource::new(self.http_client.clone())));
        self.register(Arc::new(mg::MiguSource::new(self.http_client.clone())));
    }
    
    /// 注册音乐源
    pub fn register(&self, source: Arc<dyn MusicSourceProvider>) {
        self.sources.insert(source.source_id(), source);
    }
    
    /// 获取音乐源
    pub fn get_source(
        &self,
        source_id: &MusicSource,
    ) -> Option<Arc<dyn MusicSourceProvider>> {
        self.sources.get(source_id).map(|s| Arc::clone(&*s))
    }
    
    /// 获取所有可用源
    pub fn get_available_sources(&self) -> Vec<Arc<dyn MusicSourceProvider>> {
        self.sources
            .iter()
            .map(|entry| Arc::clone(&*entry.value()))
            .collect()
    }
    
    /// 搜索（指定源）
    pub async fn search(
        &self,
        source_id: MusicSource,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MusicInfo>> {
        let source = self.get_source(&source_id)
            .ok_or_else(|| common::Error::SourceNotFound(format!("{:?}", source_id)))?;
        
        source.search(keyword, page, limit).await
    }
    
    /// 多源搜索
    pub async fn search_all(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Vec<(MusicSource, Result<Vec<MusicInfo>>)> {
        let mut results = Vec::new();
        
        for entry in self.sources.iter() {
            let source_id = entry.key().clone();
            let source = Arc::clone(&*entry.value());
            
            let result = source.search(keyword, page, limit).await;
            results.push((source_id, result));
        }
        
        results
    }
    
    /// 获取音乐 URL
    pub async fn get_music_url(
        &self,
        music_info: &MusicInfo,
        quality: MusicQuality,
    ) -> Result<String> {
        let source = self.get_source(&music_info.source)
            .ok_or_else(|| common::Error::SourceNotFound(format!("{:?}", music_info.source)))?;
        
        source.get_music_url(music_info, quality).await
    }
    
    /// 获取歌词
    pub async fn get_lyric(
        &self,
        music_info: &MusicInfo,
    ) -> Result<common::LyricInfo> {
        let source = self.get_source(&music_info.source)
            .ok_or_else(|| common::Error::SourceNotFound(format!("{:?}", music_info.source)))?;
        
        source.get_lyric(music_info).await
    }
    
    /// 跨源查找音乐
    pub async fn find_music_cross_source(
        &self,
        music_info: &MusicInfo,
    ) -> Result<Vec<MusicInfo>> {
        let keyword = format!("{} {}", music_info.name, music_info.singer.join(" "));
        let mut candidates = Vec::new();
        
        // 在所有源中搜索
        for entry in self.sources.iter() {
            let source = Arc::clone(&*entry.value());
            
            // 跳过原源
            if source.source_id() == music_info.source {
                continue;
            }
            
            match source.search(&keyword, 1, 10).await {
                Ok(results) => {
                    // 匹配算法：基于歌曲名、歌手、专辑、时长
                    for result in results {
                        if Self::is_match(music_info, &result) {
                            candidates.push(result);
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        
        Ok(candidates)
    }
    
    /// 匹配两个音乐信息是否相同
    /// 与上游 `matchMusicInfo` 同口径：名字先经 filterStr 过滤（去空白/分隔符）再转小写，
    /// 且两侧对每个版本特征词条（伴奏/现场/翻唱等）的包含结果必须一致
    fn is_match(a: &MusicInfo, b: &MusicInfo) -> bool {
        let f_name_a = utils::filter_str_lower(&a.name);
        let f_name_b = utils::filter_str_lower(&b.name);

        // 1. 歌曲名：过滤后互相包含（isIncludesName）或相似度足够高
        let name_match = !f_name_b.is_empty()
            && (f_name_a.contains(&f_name_b) || f_name_b.contains(&f_name_a)
                || utils::similarity(&f_name_a, &f_name_b) > 0.8);

        // 2. 版本特征一致性（isEqualsVersionMusicNameChar）
        let version_match = utils::is_equals_version_chars(&f_name_a, &f_name_b);

        // 3. 歌手匹配（至少有一位相同）
        let singer_match = a.singer.iter().any(|s| {
            b.singer.iter().any(|bs| {
                utils::similarity(&s.to_lowercase(), &bs.to_lowercase()) > 0.7
            })
        });

        // 4. 时长匹配（允许 ±5 秒误差，上游 isEqualsInterval 为 <=5）
        let duration_match = (a.interval as i32 - b.interval as i32).abs() <= 5;

        name_match && version_match && singer_match && duration_match
    }
}

/// 重新导出
pub use sources::kuwo;
pub use sources::kugou;
pub use sources::qq_music;
pub use sources::netease;
pub use sources::migu;

mod kw {
    pub use super::sources::kuwo::*;
}
mod kg {
    pub use super::sources::kugou::*;
}
mod tx {
    pub use super::sources::qq_music::*;
}
mod wy {
    pub use super::sources::netease::*;
}
mod mg {
    pub use super::sources::migu::*;
}
