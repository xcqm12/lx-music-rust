use crate::{MusicSourceProvider, crypto, utils};
use common::{MusicInfo, MusicQuality, MusicSource, LyricInfo, Result, Error};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

pub struct KuwoSource {
    client: Arc<Client>,
    base_url: String,
}

impl KuwoSource {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            base_url: "https://www.kuwo.cn".to_string(),
        }
    }
    
    /// 获取 CSRF Token
    async fn get_csrf(&self) -> Result<String> {
        let url = format!("{}/", self.base_url);
        let resp = self.client.get(&url).send().await?;
        
        // 从 Cookie 或响应中提取 CSRF
        Ok(utils::random_string(10))
    }
    
    /// 构建请求头
    fn build_headers(&self, csrf: &str) -> Vec<(&'static str, String)> {
        vec![
            ("csrf", csrf.to_string()),
            ("Referer", "https://www.kuwo.cn/".to_string()),
        ]
    }
}

#[async_trait::async_trait]
impl MusicSourceProvider for KuwoSource {
    fn name(&self) -> &str {
        "酷我音乐"
    }
    
    fn source_id(&self) -> MusicSource {
        MusicSource::Kw
    }
    
    async fn search(
        &self,
        keyword: &str,
        page: u32,
        limit: u32,
    ) -> Result<Vec<MusicInfo>> {
        let csrf = self.get_csrf().await?;
        let url = format!(
            "{}/api/www/search/searchMusicBykeyWord",
            self.base_url
        );
        
        let params = [
            ("key", keyword),
            ("pn", &page.to_string()),
            ("rn", &limit.to_string()),
        ];
        
        let mut headers = reqwest::header::HeaderMap::new();
        for (k, v) in self.build_headers(&csrf) {
            headers.insert(k, v.parse().unwrap());
        }
        
        let resp = self.client
            .get(&url)
            .query(&params)
            .headers(headers)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        let mut results = Vec::new();
        
        if let Some(list) = json.get("data").and_then(|d| d.get("list")).and_then(|l| l.as_array()) {
            for item in list {
                let music_info = self.parse_music_info(item)?;
                results.push(music_info);
            }
        }
        
        Ok(results)
    }
    
    async fn get_music_url(
        &self,
        music_info: &MusicInfo,
        quality: MusicQuality,
    ) -> Result<String> {
        let quality_map = match quality {
            MusicQuality::Lq => "128kmp3",
            MusicQuality::Mq => "192kmp3",
            MusicQuality::Hq => "320kmp3",
            MusicQuality::Sq => "2000kflac",
            MusicQuality::Hires => "4000kflac",
        };
        
        let url = format!(
            "{}/api/v1/www/music/playUrl",
            self.base_url
        );
        
        let params = [
            ("mid", music_info.id.as_str()),
            ("type", "music"),
            ("httpsStatus", "1"),
            ("format", "mp3|flac"),
            ("br", quality_map),
        ];
        
        let resp = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        json.get("data")
            .and_then(|d| d.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::MusicSource("Failed to get music URL".to_string()))
    }
    
    async fn get_lyric(&self, music_info: &MusicInfo) -> Result<LyricInfo> {
        let url = format!(
            "{}/api/v1/www/music/playUrl",
            self.base_url
        );
        
        let params = [
            ("mid", music_info.id.as_str()),
            ("type", "lyric"),
        ];
        
        let resp = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?;
        
        let json: Value = resp.json().await?;
        
        let lyric = json.get("data")
            .and_then(|d| d.get("lrclist"))
            .and_then(|l| l.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        
        Ok(LyricInfo {
            lyric,
            tlyric: None,
            rlyric: None,
            lxlyric: None,
        })
    }
    
    async fn get_pic_url(&self, music_info: &MusicInfo) -> Result<String> {
        // 酷我封面 URL 格式
        Ok(format!(
            "https://img4.kuwo.cn/star/albumcover/{}.jpg",
            music_info.id
        ))
    }
    
    async fn check_available(&self) -> Result<bool> {
        match self.client.get(&self.base_url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl KuwoSource {
    fn parse_music_info(&self, item: &Value) -> Result<MusicInfo> {
        let id = item.get("rid")
            .and_then(|v| v.as_i64())
            .map(|i| i.to_string())
            .ok_or_else(|| Error::ParseFailed("Missing rid".to_string()))?;
        
        let name = item.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let singer = item.get("artist")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .split(",")
            .map(|s| s.trim().to_string())
            .collect();
        
        let album_name = item.get("album")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let interval = item.get("duration")
            .and_then(|v| v.as_i64())
            .map(|i| (i / 1000) as u32)
            .unwrap_or(0);
        
        let mut quality = std::collections::BTreeMap::new();
        quality.insert(MusicQuality::Lq, id.clone());
        quality.insert(MusicQuality::Hq, id.clone());
        
        Ok(MusicInfo {
            id,
            name,
            singer,
            album_name,
            interval,
            source: MusicSource::Kw,
            quality,
            pic_url: None,
        })
    }
}
